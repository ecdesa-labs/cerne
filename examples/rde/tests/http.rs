//! The rde over JSON-RPC, like the server: the transfers and the chain (`SqliteBlockchain`) each in a SQLite in
//! memory. Each request goes straight to the router, without a socket.
//! The calls are the ones MetaMask makes (`docs/METAMASK.md`), plus the rde methods of the recipient.

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use cerne::application::OutboxPolicyProcessor;
use cerne::sqlite::SqliteDatabase;
use rde::domain::services::units::WEI_PER_RDEC;
use rde::infrastructure::http::router;
use rde::infrastructure::in_memory_kyc_registry::InMemoryKycRegistry;
use rde::infrastructure::in_memory_notifier::InMemoryNotifier;
use rde::infrastructure::local_wallet::LocalWallet;
use rde::infrastructure::sqlite_blockchain::SqliteBlockchain;
use rde::ports::{ExternalSystems, Ports, command_registry};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

// --- Fixtures ----------------------------------------------------------------

fn alice() -> LocalWallet {
    LocalWallet::new("0x0000000000000000000000000000000000000000000000000000000000000001")
}

fn bob() -> LocalWallet {
    LocalWallet::new("0x0000000000000000000000000000000000000000000000000000000000000002")
}

fn carol() -> LocalWallet {
    LocalWallet::new("0x0000000000000000000000000000000000000000000000000000000000000003")
}

/// alice has 1000 RDEC; alice and bob passed KYC, carol did not.
async fn ports() -> Arc<Ports> {
    let database = SqliteDatabase::in_memory().await.unwrap();
    database.migrate(&sqlx::migrate!()).await.unwrap();

    let chain_database = SqliteDatabase::in_memory().await.unwrap();
    let blockchain = SqliteBlockchain::open(
        chain_database,
        vec![(alice().address().clone(), 1000 * WEI_PER_RDEC)],
    )
    .await
    .unwrap();

    let external_systems = ExternalSystems {
        blockchain: Arc::new(blockchain),
        kyc: Arc::new(InMemoryKycRegistry::new(vec![
            alice().address().clone(),
            bob().address().clone(),
        ])),
        notifier: Arc::new(InMemoryNotifier::new(Arc::new(Mutex::new(vec![])))),
    };

    Arc::new(Ports::new(database, external_systems))
}

/// Sends `POST /rpc` with this body and returns the JSON-RPC response.
async fn post(router: &Router, body: Value) -> Value {
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

/// A call with `params`, by position (an array, like MetaMask) or by name (an object).
async fn rpc(router: &Router, method: &str, params: Value) -> Value {
    post(
        router,
        json!({ "jsonrpc": "2.0", "method": method, "params": params, "id": 1 }),
    )
    .await
}

// --- Lane: MetaMask sends, bob accepts, the outbox chains --------------------

#[tokio::test]
async fn alice_sends_from_metamask_bob_accepts_and_metamask_sees_the_receipt() {
    let ports = ports().await;
    let router = router(Arc::clone(&ports));
    let outbox_policy_processor =
        OutboxPolicyProcessor::new(Arc::clone(&ports), command_registry());

    // MetaMask: the nonce, then the signed transaction.
    let next_nonce = rpc(
        &router,
        "eth_getTransactionCount",
        json!([alice().address(), "latest"]),
    )
    .await;
    assert_eq!(next_nonce["result"], "0x0");

    let signed_transaction = alice().sign_transfer(bob().address(), 100 * WEI_PER_RDEC, 0);

    let sent = rpc(
        &router,
        "eth_sendRawTransaction",
        json!([signed_transaction]),
    )
    .await;
    let tx_hash = sent["result"].clone();
    assert_eq!(tx_hash, json!(signed_transaction.tx_hash()));

    let receipt = rpc(&router, "eth_getTransactionReceipt", json!([tx_hash])).await;
    assert_eq!(receipt["result"], Value::Null, "pending: in no block yet");

    // bob: the rde methods.
    let pending = rpc(
        &router,
        "pending_transfers",
        json!({ "recipient": bob().address() }),
    )
    .await;

    assert_eq!(
        pending["result"],
        json!({
            "recipient": bob().address(),
            "transfers": [{
                "tx_hash": tx_hash,
                "sender": alice().address(),
                "amount": "100000000000000000000"
            }]
        })
    );

    let accepted = rpc(
        &router,
        "accept_transfer",
        json!({ "tx_hash": tx_hash, "recipient": bob().address() }),
    )
    .await;

    assert_eq!(accepted["result"], Value::Null);

    outbox_policy_processor.run_pending().await.unwrap();

    // MetaMask: a new block, and the receipt in it.
    let block_number = rpc(&router, "eth_blockNumber", json!([])).await;
    assert_eq!(block_number["result"], "0x1");

    let receipt = rpc(&router, "eth_getTransactionReceipt", json!([tx_hash])).await;
    assert_eq!(receipt["result"]["status"], "0x1");
    assert_eq!(receipt["result"]["blockNumber"], "0x1");

    let block = rpc(
        &router,
        "eth_getBlockByHash",
        json!([receipt["result"]["blockHash"], false]),
    )
    .await;
    assert_eq!(block["result"]["number"], "0x1");
    assert_eq!(block["result"]["transactions"], json!([tx_hash]));

    let balance = rpc(&router, "eth_getBalance", json!([bob().address(), "0x1"])).await;
    assert_eq!(balance["result"], format!("0x{:x}", 100 * WEI_PER_RDEC));
}

// --- Lane: alice cancels in MetaMask -----------------------------------------

#[tokio::test]
async fn alice_cancels_in_metamask_and_metamask_follows_the_cancellation() {
    let ports = ports().await;
    let router = router(Arc::clone(&ports));
    let outbox_policy_processor =
        OutboxPolicyProcessor::new(Arc::clone(&ports), command_registry());

    let signed_transaction = alice().sign_transfer(bob().address(), 100 * WEI_PER_RDEC, 0);
    let sent = rpc(
        &router,
        "eth_sendRawTransaction",
        json!([signed_transaction]),
    )
    .await;
    let tx_hash = sent["result"].clone();

    // "Cancel": nothing to alice herself, with the same nonce. MetaMask follows the hash of the cancellation.
    let cancellation = alice().sign_cancellation(0);
    let canceled = rpc(&router, "eth_sendRawTransaction", json!([cancellation])).await;
    assert_eq!(canceled["result"], json!(cancellation.tx_hash()));

    // "Speed up" on the cancellation sends the same bytes: the same hash comes back, as on a node.
    let sent_again = rpc(&router, "eth_sendRawTransaction", json!([cancellation])).await;
    assert_eq!(sent_again["result"], json!(cancellation.tx_hash()));

    outbox_policy_processor.run_pending().await.unwrap();

    // MetaMask: the cancellation in block 1, the transfer in no block, the nonce moved on, no RDEC spent.
    let cancellation_receipt = rpc(
        &router,
        "eth_getTransactionReceipt",
        json!([cancellation.tx_hash()]),
    )
    .await;
    assert_eq!(cancellation_receipt["result"]["status"], "0x1");
    assert_eq!(cancellation_receipt["result"]["blockNumber"], "0x1");
    assert_eq!(cancellation_receipt["result"]["gasUsed"], "0x0");

    let transfer_receipt = rpc(&router, "eth_getTransactionReceipt", json!([tx_hash])).await;
    assert_eq!(transfer_receipt["result"], Value::Null);

    let next_nonce = rpc(
        &router,
        "eth_getTransactionCount",
        json!([alice().address(), "latest"]),
    )
    .await;
    assert_eq!(next_nonce["result"], "0x1");

    let balance = rpc(&router, "eth_getBalance", json!([alice().address(), "0x1"])).await;
    assert_eq!(balance["result"], format!("0x{:x}", 1000 * WEI_PER_RDEC));

    // bob has nothing left to answer.
    let pending = rpc(&router, "pending_transfers", json!([bob().address()])).await;
    assert_eq!(pending["result"]["transfers"], json!([]));
}

#[tokio::test]
async fn a_transfer_sent_again_comes_back_with_its_hash() {
    let router = router(ports().await);
    let signed_transaction = alice().sign_transfer(bob().address(), 100 * WEI_PER_RDEC, 0);

    let sent = rpc(
        &router,
        "eth_sendRawTransaction",
        json!([signed_transaction]),
    )
    .await;
    let sent_again = rpc(
        &router,
        "eth_sendRawTransaction",
        json!([signed_transaction]),
    )
    .await;

    assert_eq!(sent_again["result"], sent["result"]);
}

#[tokio::test]
async fn speed_up_on_a_pending_transfer_is_refused() {
    let router = router(ports().await);
    let signed_transaction = alice().sign_transfer(bob().address(), 100 * WEI_PER_RDEC, 0);

    rpc(
        &router,
        "eth_sendRawTransaction",
        json!([signed_transaction]),
    )
    .await;

    let speed_up = alice().sign_speed_up(bob().address(), 100 * WEI_PER_RDEC, 0);
    let response = rpc(&router, "eth_sendRawTransaction", json!([speed_up])).await;

    assert_eq!(response["error"]["code"], -32001);
    assert_eq!(
        response["error"]["data"]["violations"],
        json!(["sender has no open transfer"])
    );
}

// --- MetaMask adding the network ---------------------------------------------

#[tokio::test]
async fn the_chain_id_is_the_one_configured_in_metamask() {
    let router = router(ports().await);

    let chain_id = rpc(&router, "eth_chainId", json!([])).await;

    assert_eq!(chain_id["result"], "0x2268");
}

#[tokio::test]
async fn net_version_comes_without_params() {
    let router = router(ports().await);

    let net_version = post(
        &router,
        json!({ "jsonrpc": "2.0", "method": "net_version", "id": "hPlnKW6QX9vSHNPnnjJXX" }),
    )
    .await;

    assert_eq!(net_version["result"], "8808");
    assert_eq!(net_version["id"], "hPlnKW6QX9vSHNPnnjJXX");
}

#[tokio::test]
async fn the_genesis_block_has_a_base_fee() {
    let router = router(ports().await);

    let block = rpc(&router, "eth_getBlockByNumber", json!(["0x0", false])).await;

    assert_eq!(block["result"]["number"], "0x0");
    assert_eq!(block["result"]["baseFeePerGas"], "0x3b9aca00");
}

#[tokio::test]
async fn a_position_takes_an_object_and_the_id_comes_back_as_sent() {
    let router = router(ports().await);
    let bob = bob();
    let to = bob.address();

    let estimate_gas = post(
        &router,
        json!({
            "jsonrpc": "2.0",
            "method": "eth_estimateGas",
            "params": [{ "from": alice().address(), "to": to, "value": "0x16345785d8a0000", "data": "0x", "type": "0x2" }],
            "id": 7012491944213331_u64
        }),
    )
    .await;
    let call = rpc(
        &router,
        "eth_call",
        json!([{ "to": to, "data": "0x95d89b41" }, "0x1"]),
    )
    .await;

    assert_eq!(estimate_gas["result"], "0x5208");
    assert_eq!(estimate_gas["id"], 7012491944213331_u64);
    assert_eq!(call["result"], "0x");
}

// --- Recipient: the rde methods, by name or by position ----------------------

#[tokio::test]
async fn a_query_reads_its_params_by_name_or_by_position() {
    let router = router(ports().await);

    let by_name = rpc(
        &router,
        "pending_transfers",
        json!({ "recipient": bob().address() }),
    )
    .await;
    let by_position = rpc(&router, "pending_transfers", json!([bob().address()])).await;

    let no_transfers = json!({ "recipient": bob().address(), "transfers": [] });

    assert_eq!(by_name["result"], no_transfers);
    assert_eq!(by_position["result"], no_transfers);
}

// --- Errors ------------------------------------------------------------------

/// Sends `POST /rpc` with a raw body: the answer is still JSON-RPC, with HTTP 200 and `"id": null`.
async fn post_raw(router: &Router, body: &str) -> (u16, Value) {
    let request = Request::post("/rpc")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn a_body_that_is_not_json_is_a_parse_error() {
    let router = router(ports().await);

    let (status, response) = post_raw(&router, "{ not json").await;

    assert_eq!(status, 200);
    assert_eq!(response["error"]["code"], -32700);
    assert_eq!(response["id"], Value::Null);
}

#[tokio::test]
async fn json_without_a_method_is_an_invalid_request() {
    let router = router(ports().await);

    let (status, response) = post_raw(&router, r#"{ "jsonrpc": "2.0", "id": 1 }"#).await;

    assert_eq!(status, 200);
    assert_eq!(response["error"]["code"], -32600);
    assert_eq!(response["id"], Value::Null);
}

#[tokio::test]
async fn a_jsonrpc_other_than_2_0_is_an_invalid_request() {
    let router = router(ports().await);

    let (status, response) = post_raw(
        &router,
        r#"{ "jsonrpc": "1.0", "method": "eth_chainId", "params": [], "id": 1 }"#,
    )
    .await;

    assert_eq!(status, 200);
    assert_eq!(response["error"]["code"], -32600);
}

#[tokio::test]
async fn a_broken_business_rule_answers_with_the_violations() {
    let router = router(ports().await);
    let signed_transaction = alice().sign_transfer(carol().address(), 2000 * WEI_PER_RDEC, 0);

    let response = rpc(
        &router,
        "eth_sendRawTransaction",
        json!([signed_transaction]),
    )
    .await;

    assert_eq!(
        response["error"],
        json!({
            "code": -32001,
            "message": "the domain refused the request: sender has RDEC available, recipient has KYC",
            "data": { "violations": ["sender has RDEC available", "recipient has KYC"] }
        })
    );
}

#[tokio::test]
async fn bytes_that_are_not_a_signed_transaction_are_invalid_params() {
    let router = router(ports().await);

    let response = rpc(&router, "eth_sendRawTransaction", json!(["0x1234"])).await;

    assert_eq!(response["error"]["code"], -32602);
}

#[tokio::test]
async fn a_tx_hash_that_breaks_its_invariants_is_invalid_params() {
    let router = router(ports().await);

    let response = rpc(
        &router,
        "accept_transfer",
        json!({ "tx_hash": "hash", "recipient": bob().address() }),
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
        json!({ "tx_hash": tx_hash, "recipient": bob().address() }),
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
