//! The outbox on SQLite in memory: place order → OrderPlaced → "reserve stock" → reserve stock → StockReserved →
//! "ship order" → ship order.
#![cfg(feature = "sqlite")]

use cerne::application::{
    Command, CommandRegistry, CommandRun, Executed, Outbox, OutboxPolicyProcessor,
    TransactionalPorts,
};
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, Policy};
use cerne::sqlite::{SqliteDatabase, SqliteOutbox};
use cerne::{DomainError, Error, async_trait};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;

// --- Ports -------------------------------------------------------------------

struct Ports {
    database: SqliteDatabase,
    outbox: SqliteOutbox,
}

impl Ports {
    fn new(database: SqliteDatabase) -> Self {
        Self {
            outbox: SqliteOutbox::new(database.clone()),
            database,
        }
    }

    /// Writes one line of the log, the stand-in for every table of the application.
    async fn log(&self, line: &str) -> Result<(), Error> {
        let insert = sqlx::query("INSERT INTO log (line) VALUES ($1)").bind(line.to_string());

        self.database.execute(insert).await?;

        Ok(())
    }
}

#[async_trait]
impl TransactionalPorts for Ports {
    async fn begin(&self) -> Result<Self, Error> {
        let transaction = self.database.begin().await?;

        Ok(Ports::new(transaction))
    }

    async fn commit(self) -> Result<(), Error> {
        self.database.commit().await
    }

    fn outbox(&self) -> &dyn Outbox<Self> {
        &self.outbox
    }
}

async fn ports() -> Arc<Ports> {
    let database = SqliteDatabase::in_memory().await.unwrap();

    for create_table in [
        "CREATE TABLE log (line TEXT NOT NULL)",
        "CREATE TABLE cerne_outbox (
            id INTEGER PRIMARY KEY,
            command TEXT NOT NULL,
            json TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            error TEXT
        )",
    ] {
        database.execute(sqlx::query(create_table)).await.unwrap();
    }

    Arc::new(Ports::new(database))
}

async fn log(ports: &Ports) -> Vec<String> {
    let rows = ports
        .database
        .fetch_all(sqlx::query("SELECT line FROM log ORDER BY rowid"))
        .await
        .unwrap();

    rows.iter().map(|row| row.get("line")).collect()
}

async fn outbox_statuses(ports: &Ports) -> Vec<(String, String)> {
    let rows = ports
        .database
        .fetch_all(sqlx::query(
            "SELECT command, status FROM cerne_outbox ORDER BY id",
        ))
        .await
        .unwrap();

    rows.iter()
        .map(|row| (row.get("command"), row.get("status")))
        .collect()
}

fn command_registry() -> CommandRegistry<Ports> {
    CommandRegistry::new()
        .register::<ReserveStockCommand>()
        .register::<ShipOrderCommand>()
}

// --- Commands, events and policies -------------------------------------------

struct PlaceOrderCommand;

#[derive(Serialize, Deserialize)]
struct ReserveStockCommand {
    sku: String,
}

#[derive(Serialize, Deserialize)]
struct ShipOrderCommand {
    fails: bool,
}

struct OrderPlaced;

struct StockReserved;

#[async_trait]
impl Command<Ports> for PlaceOrderCommand {
    type Output = ();

    async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
        ports.log("order placed").await?;

        let order_placed = OrderPlaced;

        Ok(Executed {
            output: (),
            events: vec![Box::new(order_placed)],
        })
    }
}

#[async_trait]
impl Command<Ports> for ReserveStockCommand {
    type Output = ();

    async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
        ports.log(&format!("{} reserved", self.sku)).await?;

        let stock_reserved = StockReserved;

        Ok(Executed {
            output: (),
            events: vec![Box::new(stock_reserved)],
        })
    }
}

#[async_trait]
impl Command<Ports> for ShipOrderCommand {
    type Output = ();

    async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
        ports.log("order shipped").await?;

        if self.fails {
            Err(DomainError::Violations(vec!["carrier is available"]))?;
        }

        Ok(Executed {
            output: (),
            events: vec![],
        })
    }
}

impl DomainEvent<Ports> for OrderPlaced {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        Ok(Policies::new(vec![Policy::new(
            "reserve stock",
            || true,
            || Box::new(ReserveStockCommand { sku: "book".into() }),
        )])
        .trigger())
    }
}

impl DomainEvent<Ports> for StockReserved {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        Ok(Policies::new(vec![Policy::new(
            "ship order",
            || true,
            || Box::new(ShipOrderCommand { fails: false }),
        )])
        .trigger())
    }
}

/// Places the order in a transaction and stores the commands of its policies in the same one.
async fn place_order(ports: &Ports) -> Vec<&'static str> {
    let transaction = ports.begin().await.unwrap();

    let place_order_execution = PlaceOrderCommand.execute(&transaction).await.unwrap();

    let fired_policies = transaction
        .outbox
        .send_events(place_order_execution.events)
        .await
        .unwrap();

    transaction.commit().await.unwrap();

    fired_policies
}

fn done(command: &str) -> CommandRun {
    CommandRun {
        command: command.into(),
        error: None,
    }
}

// --- Transaction -------------------------------------------------------------

#[tokio::test]
async fn the_aggregate_and_the_outbox_commit_together() {
    let ports = ports().await;

    let fired_policies = place_order(&ports).await;

    assert_eq!(fired_policies, vec!["reserve stock"]);
    assert_eq!(log(&ports).await, vec!["order placed"]);
    assert_eq!(
        outbox_statuses(&ports).await,
        vec![("reserve_stock".into(), "pending".into())]
    );
}

#[tokio::test]
async fn without_commit_neither_the_aggregate_nor_the_outbox_is_written() {
    let ports = ports().await;

    {
        let transaction = ports.begin().await.unwrap();
        let place_order_execution = PlaceOrderCommand.execute(&transaction).await.unwrap();
        transaction
            .outbox
            .send_events(place_order_execution.events)
            .await
            .unwrap();
    }

    assert!(log(&ports).await.is_empty());
    assert!(outbox_statuses(&ports).await.is_empty());
}

// --- Outbox policy processor -------------------------------------------------

#[tokio::test]
async fn run_pending_runs_the_whole_chain_through_the_outbox() {
    let ports = ports().await;
    let outbox_policy_processor =
        OutboxPolicyProcessor::new(Arc::clone(&ports), command_registry());
    place_order(&ports).await;

    let command_runs = outbox_policy_processor.run_pending().await.unwrap();

    assert_eq!(
        command_runs,
        vec![done("reserve_stock"), done("ship_order")]
    );
    assert_eq!(
        log(&ports).await,
        vec!["order placed", "book reserved", "order shipped"]
    );
    assert_eq!(
        outbox_statuses(&ports).await,
        vec![
            ("reserve_stock".into(), "done".into()),
            ("ship_order".into(), "done".into())
        ]
    );
}

#[tokio::test]
async fn a_failing_command_rolls_back_and_is_marked_failed() {
    let ports = ports().await;
    let outbox_policy_processor =
        OutboxPolicyProcessor::new(Arc::clone(&ports), command_registry());
    let ship_order = r#"{"fails":true}"#;
    ports
        .database
        .execute(
            sqlx::query("INSERT INTO cerne_outbox (command, json) VALUES ('ship_order', $1)")
                .bind(ship_order),
        )
        .await
        .unwrap();

    let command_runs = outbox_policy_processor.run_pending().await.unwrap();

    assert_eq!(
        command_runs,
        vec![CommandRun {
            command: "ship_order".into(),
            error: Some(r#"violated: ["carrier is available"]"#.into()),
        }]
    );
    assert!(log(&ports).await.is_empty(), "the log line was rolled back");
    assert_eq!(
        outbox_statuses(&ports).await,
        vec![("ship_order".into(), "failed".into())]
    );
    assert!(
        outbox_policy_processor
            .run_pending()
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn a_command_missing_from_the_registry_is_marked_failed() {
    let ports = ports().await;
    let outbox_policy_processor =
        OutboxPolicyProcessor::new(Arc::clone(&ports), CommandRegistry::new());
    place_order(&ports).await;

    let command_runs = outbox_policy_processor.run_pending().await.unwrap();

    assert_eq!(
        command_runs[0].error.as_deref(),
        Some("reserve_stock is not in the command registry")
    );
}
