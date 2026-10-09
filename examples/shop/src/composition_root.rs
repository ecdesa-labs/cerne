use crate::application::ports::catalog::Catalog;
use crate::application::ports::payments::Payments;
use crate::domain::entities::order::Order;
use crate::infrastructure::database::{Database, Transaction};
use cerne::application::{EventOutbox, Repository};
use std::sync::Arc;

/// The composition root: the database and every port the commands and queries can use.
pub struct CompositionRoot {
    pub database: Database,
    pub order_repository: Box<dyn Repository<Order, Transaction>>,
    pub event_outbox: Box<dyn EventOutbox<Transaction>>,
    pub catalog: Arc<dyn Catalog>,
    pub payments: Arc<dyn Payments>,
}

/// What `CompositionRoot::new` takes: every adapter, already built.
pub struct CompositionRootConstructor {
    pub database: Database,
    pub order_repository: Box<dyn Repository<Order, Transaction>>,
    pub event_outbox: Box<dyn EventOutbox<Transaction>>,
    pub catalog: Arc<dyn Catalog>,
    pub payments: Arc<dyn Payments>,
}

impl CompositionRoot {
    pub fn new(constructor: CompositionRootConstructor) -> Self {
        Self {
            database: constructor.database,
            order_repository: constructor.order_repository,
            event_outbox: constructor.event_outbox,
            catalog: constructor.catalog,
            payments: constructor.payments,
        }
    }
}
