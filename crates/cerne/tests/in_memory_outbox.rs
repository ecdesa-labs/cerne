//! The outbox of a project without a database: place order → OrderPlaced → "reserve stock" → reserve stock, with the
//! commands of the policies waiting in memory.

use cerne::application::{
    Command, CommandRegistry, CommandRun, Executed, InMemoryOutbox, Outbox, OutboxPolicyProcessor, TransactionalPorts,
};
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, policy};
use cerne::{DomainError, Error, async_trait};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

// --- Ports -------------------------------------------------------------------

struct Ports {
    in_memory_outbox: InMemoryOutbox,
    outbox: Box<dyn Outbox<Ports>>,
    warehouse: Arc<Mutex<Vec<String>>>,
}

impl Ports {
    fn new(in_memory_outbox: InMemoryOutbox, warehouse: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            outbox: Box::new(in_memory_outbox.clone()),
            in_memory_outbox,
            warehouse,
        }
    }
}

#[async_trait]
impl TransactionalPorts for Ports {
    async fn begin(&self) -> Result<Self, Error> {
        let transaction = self.in_memory_outbox.begin().await?;
        let warehouse = Arc::clone(&self.warehouse);

        Ok(Ports::new(transaction, warehouse))
    }

    async fn commit(self) -> Result<(), Error> {
        self.in_memory_outbox.commit().await
    }

    fn outbox(&self) -> &dyn Outbox<Self> {
        self.outbox.as_ref()
    }
}

fn command_registry() -> CommandRegistry<Ports> {
    CommandRegistry::new().register::<ReserveStockCommand>()
}

// --- Commands, events and policies -------------------------------------------

struct PlaceOrderCommand {
    sku: String,
}

#[derive(Serialize, Deserialize)]
struct ReserveStockCommand {
    sku: String,
}

struct OrderPlaced {
    sku: String,
}

#[async_trait]
impl Command<Ports> for PlaceOrderCommand {
    type Output = ();

    async fn execute(&self, _ports: &Ports) -> Result<Executed<(), Ports>, Error> {
        let order_placed = OrderPlaced {
            sku: self.sku.clone(),
        };

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
        let sku_exists = self.sku != "missing";

        if !sku_exists {
            Err(DomainError::Violations(vec!["sku exists"]))?;
        }

        ports.warehouse.lock().unwrap().push(self.sku.clone());

        Ok(Executed {
            output: (),
            events: vec![],
        })
    }
}

impl DomainEvent<Ports> for OrderPlaced {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        // --- Policies --------------------------------------------------------

        let sku = self.sku.clone();

        let reserve_stock_policy =
            policy!("whenever an order is placed, reserve stock", true, ReserveStockCommand { sku });

        Ok(Policies::trigger([reserve_stock_policy]))
    }
}

// --- Tests -------------------------------------------------------------------

#[tokio::test]
async fn the_outbox_in_memory_runs_the_commands_of_the_policies() -> Result<(), Error> {
    let in_memory_outbox = InMemoryOutbox::new();
    let warehouse = Arc::new(Mutex::new(vec![]));
    let ports = Arc::new(Ports::new(in_memory_outbox.clone(), Arc::clone(&warehouse)));

    let outbox_policy_processor = OutboxPolicyProcessor::new(Arc::clone(&ports), command_registry());

    // --- Customer: places two orders -----------------------------------------

    let mug = PlaceOrderCommand { sku: "mug".into() };
    let missing = PlaceOrderCommand {
        sku: "missing".into(),
    };

    ports.execute_in_transaction(mug).await?;
    ports.execute_in_transaction(missing).await?;

    assert!(warehouse.lock().unwrap().is_empty(), "the policies only stored their commands");

    // --- Policy: whenever an order is placed, reserve stock ------------------

    let command_runs = outbox_policy_processor.run_pending().await?;

    let reserved_and_failed = vec![
        CommandRun {
            command: "reserve_stock".into(),
            error: None,
        },
        CommandRun {
            command: "reserve_stock".into(),
            error: Some(r#"violated: ["sku exists"]"#.into()),
        },
    ];

    assert_eq!(command_runs, reserved_and_failed);
    assert_eq!(*warehouse.lock().unwrap(), vec!["mug".to_string()]);
    assert_eq!(in_memory_outbox.failures().len(), 1);
    assert!(outbox_policy_processor.run_pending().await?.is_empty(), "nothing runs twice");

    Ok(())
}
