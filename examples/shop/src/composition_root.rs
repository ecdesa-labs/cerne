use crate::application::commands::charge_order::ChargeOrderCommand;
use crate::application::ports::catalog::Catalog;
use crate::application::ports::payments::Payments;
use crate::domain::entities::order::Order;
use crate::infrastructure::sqlite_order_repository::SqliteOrderRepository;
use cerne::application::{CommandRegistry, Outbox, Repository, TransactionalCompositionRoot};
use cerne::sqlite::{SqliteDatabase, SqliteOutbox};
use cerne::{Error, async_trait};
use std::sync::Arc;

/// The composition root: every port the commands and queries can use.
///
/// The repositories and the outbox live in the database, so they follow its transaction. External systems do not:
/// `begin` hands the same adapters to the new composition root.
pub struct CompositionRoot {
    pub database: SqliteDatabase,
    pub order_repository: Box<dyn Repository<Order>>,
    pub outbox: Box<dyn Outbox<CompositionRoot>>,
    pub catalog: Arc<dyn Catalog>,
    pub payments: Arc<dyn Payments>,
}

/// What `CompositionRoot::new` takes: the adapters the composition root is built from.
pub struct CompositionRootConstructor {
    pub database: SqliteDatabase,
    pub catalog: Arc<dyn Catalog>,
    pub payments: Arc<dyn Payments>,
}

impl CompositionRoot {
    pub fn new(constructor: CompositionRootConstructor) -> Self {
        Self {
            order_repository: Box::new(SqliteOrderRepository::new(constructor.database.clone())),
            outbox: Box::new(SqliteOutbox::new(constructor.database.clone())),
            database: constructor.database,
            catalog: constructor.catalog,
            payments: constructor.payments,
        }
    }
}

/// Every command a policy fires, so the outbox can read it back from its row.
pub fn command_registry() -> CommandRegistry<CompositionRoot> {
    CommandRegistry::new().register::<ChargeOrderCommand>()
}

#[async_trait]
impl TransactionalCompositionRoot for CompositionRoot {
    async fn begin(&self) -> Result<Self, Error> {
        let transaction = self.database.begin().await?;

        let composition_root_constructor = CompositionRootConstructor {
            database: transaction,
            catalog: Arc::clone(&self.catalog),
            payments: Arc::clone(&self.payments),
        };

        Ok(CompositionRoot::new(composition_root_constructor))
    }

    async fn commit(self) -> Result<(), Error> {
        self.database.commit().await
    }

    fn outbox(&self) -> &dyn Outbox<Self> {
        self.outbox.as_ref()
    }
}
