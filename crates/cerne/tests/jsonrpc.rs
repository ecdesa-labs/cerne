//! JSON-RPC 2.0: the body becomes a `Request` or an error, and the `params` come by name or by position.
#![cfg(all(feature = "axum", feature = "sqlite"))]

use cerne::application::{Command, Executed, Outbox, Query, ReadModel, TransactionalCompositionRoot};
use cerne::domain::{BusinessRules, business_rule};
use cerne::http::jsonrpc::{ErrorObject, Methods, Request};
use cerne::sqlite::{SqliteDatabase, SqliteOutbox};
use cerne::{Error, async_trait};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

// --- Ports -------------------------------------------------------------------

struct CompositionRoot {
    database: SqliteDatabase,
    outbox: SqliteOutbox,
}

impl CompositionRoot {
    fn new(database: SqliteDatabase) -> Self {
        Self {
            outbox: SqliteOutbox::new(database.clone()),
            database,
        }
    }
}

#[async_trait]
impl TransactionalCompositionRoot for CompositionRoot {
    async fn begin(&self) -> Result<Self, Error> {
        let transaction = self.database.begin().await?;

        Ok(CompositionRoot::new(transaction))
    }

    async fn commit(self) -> Result<(), Error> {
        self.database.commit().await
    }

    fn outbox(&self) -> &dyn Outbox<Self> {
        &self.outbox
    }
}

async fn composition_root() -> CompositionRoot {
    let database = SqliteDatabase::in_memory().await.unwrap();

    CompositionRoot::new(database)
}

// --- Command and query -------------------------------------------------------

#[derive(Deserialize)]
struct SendMoneyCommand {
    sender: String,
    recipient: String,
    amount: u64,
}

#[async_trait]
impl Command<CompositionRoot> for SendMoneyCommand {
    type Output = String;

    async fn execute(&self, _ports: &CompositionRoot) -> Result<Executed<String, CompositionRoot>, Error> {
        // --- Business rules --------------------------------------------------

        let amount_is_positive = self.amount > 0;
        let sender_is_not_recipient = self.sender != self.recipient;

        BusinessRules::check([
            business_rule!("amount is positive", amount_is_positive),
            business_rule!("sender is not recipient", sender_is_not_recipient),
        ])?;

        // --- Domain events ---------------------------------------------------

        let receipt = format!("{} sent {} to {}", self.sender, self.amount, self.recipient);

        Ok(Executed {
            output: receipt,
            events: vec![],
        })
    }
}

#[derive(Deserialize)]
struct BalanceQuery {
    owner: String,
}

#[derive(Serialize)]
struct Balance {
    owner: String,
    amount: u64,
}

impl ReadModel for Balance {}

#[async_trait]
impl Query<CompositionRoot> for BalanceQuery {
    type ReadModel = Balance;

    async fn execute(&self, _ports: &CompositionRoot) -> Result<Balance, Error> {
        Ok(Balance {
            owner: self.owner.clone(),
            amount: 1000,
        })
    }
}

// --- The params --------------------------------------------------------------

#[tokio::test]
async fn a_command_reads_its_params_by_name_or_by_position() {
    let composition_root = composition_root().await;
    let methods = Methods::new(&composition_root);

    let by_name = json!({ "sender": "alice", "recipient": "bob", "amount": 100 });
    let by_position = json!(["alice", "bob", 100]);

    let sent_by_name = methods.command::<SendMoneyCommand>(by_name).await;
    let sent_by_position = methods.command::<SendMoneyCommand>(by_position).await;

    assert_eq!(sent_by_name, Ok(json!("alice sent 100 to bob")));
    assert_eq!(sent_by_position, Ok(json!("alice sent 100 to bob")));
}

#[tokio::test]
async fn a_query_reads_its_params_by_name_or_by_position() {
    let composition_root = composition_root().await;
    let methods = Methods::new(&composition_root);

    let by_name = methods
        .query::<BalanceQuery>(json!({ "owner": "alice" }))
        .await;
    let by_position = methods.query::<BalanceQuery>(json!(["alice"])).await;

    assert_eq!(by_name, Ok(json!({ "owner": "alice", "amount": 1000 })));
    assert_eq!(by_position, Ok(json!({ "owner": "alice", "amount": 1000 })));
}

#[tokio::test]
async fn params_that_are_neither_an_array_nor_an_object_are_invalid() {
    let composition_root = composition_root().await;
    let methods = Methods::new(&composition_root);

    for params in [json!("alice"), json!(100), json!(true)] {
        let error = methods
            .command::<SendMoneyCommand>(params)
            .await
            .unwrap_err();

        assert_eq!(error.code, -32602);
    }

    let too_short = methods
        .command::<SendMoneyCommand>(json!(["alice", "bob"]))
        .await;

    assert_eq!(too_short.unwrap_err().code, -32602);
}

#[tokio::test]
async fn the_violations_go_in_the_message_that_a_wallet_shows() {
    let composition_root = composition_root().await;
    let methods = Methods::new(&composition_root);

    let error = methods
        .command::<SendMoneyCommand>(json!(["alice", "alice", 0]))
        .await
        .unwrap_err();

    assert_eq!(
        error,
        ErrorObject {
            code: -32001,
            message: "the domain refused the request: amount is positive, sender is not recipient".into(),
            data: Some(json!({ "violations": ["amount is positive", "sender is not recipient"] })),
        }
    );
}

// --- The body ----------------------------------------------------------------

fn error_code(body: &str) -> i64 {
    Request::from_body(body.as_bytes()).unwrap_err().code
}

#[test]
fn a_body_that_is_not_json_is_a_parse_error() {
    assert_eq!(error_code("{ not json"), -32700);
}

#[test]
fn json_without_a_method_is_an_invalid_request() {
    assert_eq!(error_code(r#"{ "jsonrpc": "2.0", "id": 1 }"#), -32600);
}

#[test]
fn a_jsonrpc_other_than_2_0_is_an_invalid_request() {
    let jsonrpc_1 = r#"{ "jsonrpc": "1.0", "method": "send_money", "id": 1 }"#;
    let no_jsonrpc = r#"{ "method": "send_money", "id": 1 }"#;

    assert_eq!(error_code(jsonrpc_1), -32600);
    assert_eq!(error_code(no_jsonrpc), -32600);
}

#[test]
fn a_jsonrpc_2_0_body_is_a_request() {
    let body = r#"{ "jsonrpc": "2.0", "method": "send_money", "params": ["alice", "bob", 100], "id": 7 }"#;

    let request = Request::from_body(body.as_bytes()).unwrap();

    assert_eq!(request.method, "send_money");
    assert_eq!(request.params, json!(["alice", "bob", 100]));
    assert_eq!(request.id, Value::from(7));
}
