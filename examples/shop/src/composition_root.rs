use crate::application::ports::catalog::Catalog;
use crate::application::ports::payments::Payments;
use crate::domain::entities::order::Order;
use crate::infrastructure::in_memory_database::{InMemoryDatabase, InMemoryEventOutbox, InMemoryOrderRepository};
use cerne::Error;
use cerne::application::{EventOutbox, Repository};
use std::sync::Arc;

/// The composition root: every port the commands and queries can use.
pub struct CompositionRoot {
    pub database: InMemoryDatabase,
    pub order_repository: Box<dyn Repository<Order>>,
    pub event_outbox: Box<dyn EventOutbox>,
    pub catalog: Arc<dyn Catalog>,
    pub payments: Arc<dyn Payments>,
}

/// What `CompositionRoot::new` takes: every adapter, already built.
pub struct CompositionRootConstructor {
    pub database: InMemoryDatabase,
    pub order_repository: Box<dyn Repository<Order>>,
    pub event_outbox: Box<dyn EventOutbox>,
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

    /// In memory there is no transaction: the repository and the event outbox are built again on the same data.
    pub async fn begin(&self) -> Result<CompositionRoot, Error> {
        let transaction = self.database.clone();

        let order_repository = InMemoryOrderRepository::new(transaction.clone());
        let event_outbox = InMemoryEventOutbox::new(transaction.clone());

        let composition_root_constructor = CompositionRootConstructor {
            database: transaction,
            order_repository: Box::new(order_repository),
            event_outbox: Box::new(event_outbox),
            catalog: Arc::clone(&self.catalog),
            payments: Arc::clone(&self.payments),
        };

        Ok(CompositionRoot::new(composition_root_constructor))
    }

    /// Nothing to make permanent: every write is already in memory.
    pub async fn commit(self) -> Result<(), Error> {
        Ok(())
    }
}
