use crate::domain::entities::order::Order;
use crate::domain::value_objects::order_id::OrderId;
use cerne::application::{EventOutbox, OutboxEntry, Repository};
use cerne::domain::Entity;
use cerne::{ApplicationError, Error, async_trait};
use std::sync::{Arc, Mutex};

/// What a database would hold, in memory: cloning it shares the same data. There is no transaction: a command that
/// fails halfway keeps what it already wrote.
#[derive(Clone, Default)]
pub struct InMemoryDatabase {
    pub orders: Arc<Mutex<Vec<Order>>>,
    pub event_outbox: Arc<Mutex<Vec<OutboxEntry>>>,
}

// --- Repository<Order> -------------------------------------------------------

pub struct InMemoryOrderRepository {
    database: InMemoryDatabase,
}

impl InMemoryOrderRepository {
    pub fn new(database: InMemoryDatabase) -> Self {
        Self { database }
    }
}

#[async_trait]
impl Repository<Order> for InMemoryOrderRepository {
    async fn load(&self, order_id: &OrderId) -> Result<Order, Error> {
        let orders = self.database.orders.lock().unwrap();
        let order = orders.iter().find(|order| order.id() == Some(order_id));

        Ok(order.cloned().ok_or(ApplicationError::NotFound("order"))?)
    }

    async fn save(&self, order: Order) -> Result<OrderId, Error> {
        let mut orders = self.database.orders.lock().unwrap();

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

pub struct InMemoryEventOutbox {
    database: InMemoryDatabase,
}

impl InMemoryEventOutbox {
    pub fn new(database: InMemoryDatabase) -> Self {
        Self { database }
    }
}

#[async_trait]
impl EventOutbox for InMemoryEventOutbox {
    async fn store(&self, outbox_entry: OutboxEntry) -> Result<(), Error> {
        self.database
            .event_outbox
            .lock()
            .unwrap()
            .push(outbox_entry);

        Ok(())
    }
}
