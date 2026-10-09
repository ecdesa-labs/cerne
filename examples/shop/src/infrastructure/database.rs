use crate::domain::entities::order::Order;
use crate::domain::value_objects::order_id::OrderId;
use cerne::application::{EventOutbox, OutboxEntry, Repository};
use cerne::domain::Entity;
use cerne::{ApplicationError, Error, async_trait};
use std::sync::{Arc, Mutex};

/// What a database would hold, in memory.
#[derive(Default)]
pub struct Database {
    pub orders: Arc<Mutex<Vec<Order>>>,
    pub event_outbox: Arc<Mutex<Vec<OutboxEntry>>>,
}

/// In memory there is no real transaction: every write goes straight to the `Database`, and a command that fails
/// halfway keeps what it already wrote.
pub struct Transaction {
    orders: Arc<Mutex<Vec<Order>>>,
    event_outbox: Arc<Mutex<Vec<OutboxEntry>>>,
}

impl Database {
    pub async fn begin(&self) -> Result<Transaction, Error> {
        let transaction = Transaction {
            orders: Arc::clone(&self.orders),
            event_outbox: Arc::clone(&self.event_outbox),
        };

        Ok(transaction)
    }
}

impl Transaction {
    /// Nothing to make permanent: every write is already in the `Database`.
    pub async fn commit(self) -> Result<(), Error> {
        Ok(())
    }
}

// --- Repository<Order> -------------------------------------------------------

pub struct InMemoryOrderRepository;

#[async_trait]
impl Repository<Order, Transaction> for InMemoryOrderRepository {
    async fn load(&self, transaction: &mut Transaction, order_id: &OrderId) -> Result<Order, Error> {
        let orders = transaction.orders.lock().unwrap();
        let order = orders.iter().find(|order| order.id() == Some(order_id));

        Ok(order.cloned().ok_or(ApplicationError::NotFound("order"))?)
    }

    async fn save(&self, transaction: &mut Transaction, order: Order) -> Result<OrderId, Error> {
        let mut orders = transaction.orders.lock().unwrap();

        let order_id = match order.id() {
            Some(order_id) => order_id.clone(),
            None => OrderId::try_from(orders.len() as u64 + 1)?,
        };

        orders.retain(|saved_order| saved_order.id() != Some(&order_id));
        orders.push(order.with_id(order_id.clone()));

        Ok(order_id)
    }
}

// --- EventOutbox -------------------------------------------------------------

pub struct InMemoryEventOutbox;

#[async_trait]
impl EventOutbox<Transaction> for InMemoryEventOutbox {
    async fn store(&self, transaction: &mut Transaction, outbox_entry: OutboxEntry) -> Result<(), Error> {
        transaction.event_outbox.lock().unwrap().push(outbox_entry);

        Ok(())
    }
}
