//! The rde over JSON-RPC, with SQLite in memory: each request goes straight to the router, without a socket.

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use cerne::application::OutboxPolicyProcessor;
use cerne::sqlite::SqliteDatabase;
use rde::infrastructure::http::router;
use rde::infrastructure::in_memory_blockchain::InMemoryBlockchain;
use rde::infrastructure::in_memory_kyc_registry::InMemoryKycRegistry;
use rde::infrastructure::in_memory_notifier::InMemoryNotifier;
use rde::ports::{ExternalSystems, Ports, command_registry};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

// --- Fixtures ----------------------------------------------------------------

/// alice has 1000 RDEC; alice and bob passed KYC, carol did not.
async fn ports() -> Arc<Ports> {
    let database = SqliteDatabase::in_memory().await.unwrap();
    database.migrate(&sqlx::migrate!()).await.unwrap();

    let external_systems = ExternalSystems {
        blockchain: Arc::new(InMemoryBlockchain::new(vec![("alice", 1000)])),
        kyc: Arc::new(InMemoryKycRegistry::new(vec!["alice", "bob"])),
        notifier: Arc::new(InMemoryNotifier::new(Arc::new(Mutex::new(vec![])))),
    };

    Arc::new(Ports::new(database, external_systems))
}

/// Sends `POST /rpc` and returns the JSON-RPC response.
async fn rpc(router: &Router, method: &str, params: Value) -> Value {
    let body = json!({ "jsonrpc": "2.0", "method": method, "params": params, "id": 1 });

    let request = Request::post("/rpc")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = router.clone().oneshot(request).await.unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    serde_json::from_slice(&bytes).unwrap()
}

// --- Lane: creation, response and chaining -----------------------------------

#[tokio::test]
async fn alice_creates_bob_finds_it_and_accepts_and_the_outbox_chains_it() {
    let ports = ports().await;
    let router = router(Arc::clone(&ports));
    let outbox_policy_processor =
        OutboxPolicyProcessor::new(Arc::clone(&ports), command_registry());

    let created = rpc(
        &router,
        "create_transfer",
        json!({ "sender": "alice", "recipient": "bob", "amount": 100 }),
    )
    .await;
    let tx_hash = created["result"].clone();

    let pending = rpc(&router, "pending_transfers", json!({ "recipient": "bob" })).await;

    assert_eq!(
        pending["result"],
        json!({
            "recipient": "bob",
            "transfers": [{ "tx_hash": tx_hash, "sender": "alice", "amount": 100 }]
        })
    );

    let accepted = rpc(
        &router,
        "accept_transfer",
        json!({ "tx_hash": tx_hash, "recipient": "bob" }),
    )
    .await;

    assert_eq!(accepted["result"], Value::Null);

    outbox_policy_processor.run_pending().await.unwrap();

    assert_eq!(ports.blockchain.available_rdec("bob").await.unwrap(), 100);
}

// --- Errors ------------------------------------------------------------------

#[tokio::test]
async fn a_broken_business_rule_answers_with_the_violations() {
    let router = router(ports().await);

    let response = rpc(
        &router,
        "create_transfer",
        json!({ "sender": "alice", "recipient": "carol", "amount": 2000 }),
    )
    .await;

    assert_eq!(
        response["error"],
        json!({
            "code": -32001,
            "message": "the domain refused the request",
            "data": { "violations": ["sender has RDEC available", "recipient has KYC"] }
        })
    );
}

#[tokio::test]
async fn a_tx_hash_that_breaks_its_invariants_is_invalid_params() {
    let router = router(ports().await);

    let response = rpc(
        &router,
        "accept_transfer",
        json!({ "tx_hash": "hash", "recipient": "bob" }),
    )
    .await;

    assert_eq!(response["error"]["code"], -32602);
}

#[tokio::test]
async fn an_unknown_transfer_is_not_found() {
    let router = router(ports().await);
    let tx_hash = format!("0x{}", "ab".repeat(32));

    let response = rpc(
        &router,
        "accept_transfer",
        json!({ "tx_hash": tx_hash, "recipient": "bob" }),
    )
    .await;

    assert_eq!(
        response["error"],
        json!({ "code": -32004, "message": "transfer not found" })
    );
}

#[tokio::test]
async fn an_unknown_method_is_method_not_found() {
    let router = router(ports().await);

    let response = rpc(&router, "steal_transfer", json!({})).await;

    assert_eq!(response["error"]["code"], -32601);
}
