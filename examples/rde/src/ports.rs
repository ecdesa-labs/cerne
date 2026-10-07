use crate::application::commands::chain_accepted_transfer::ChainAcceptedTransferCommand;
use crate::application::commands::notify_recipient::NotifyRecipientCommand;
use crate::application::ports::blockchain::Blockchain;
use crate::application::ports::kyc_registry::KycRegistry;
use crate::application::ports::notifier::Notifier;
use crate::application::ports::transfer_read_models::TransferReadModels;
use crate::domain::entities::transfer::Transfer;
use crate::infrastructure::sqlite_transfer_read_models::SqliteTransferReadModels;
use crate::infrastructure::sqlite_transfer_repository::SqliteTransferRepository;
use cerne::application::{CommandRegistry, Outbox, Repository, TransactionalPorts};
use cerne::sqlite::{SqliteDatabase, SqliteOutbox};
use cerne::{Error, async_trait};
use std::sync::Arc;

/// The composition root: every port the transfer commands and queries can use.
///
/// The repository, the read models and the outbox live in the database, so they follow its transaction. The external
/// systems do not: a transaction shares the same adapters.
pub struct Ports {
    pub database: SqliteDatabase,
    pub transfers: Box<dyn Repository<Transfer>>,
    pub transfer_read_models: Box<dyn TransferReadModels>,
    pub outbox: Box<dyn Outbox<Ports>>,
    pub blockchain: Arc<dyn Blockchain>,
    pub kyc: Arc<dyn KycRegistry>,
    pub notifier: Arc<dyn Notifier>,
}

/// Every command a policy fires, so the outbox can read it back from its row.
pub fn command_registry() -> CommandRegistry<Ports> {
    CommandRegistry::new()
        .register::<NotifyRecipientCommand>()
        .register::<ChainAcceptedTransferCommand>()
}

/// The ports outside the database.
pub struct ExternalSystems {
    pub blockchain: Arc<dyn Blockchain>,
    pub kyc: Arc<dyn KycRegistry>,
    pub notifier: Arc<dyn Notifier>,
}

impl Ports {
    pub fn new(database: SqliteDatabase, external_systems: ExternalSystems) -> Self {
        Self {
            transfers: Box::new(SqliteTransferRepository::new(database.clone())),
            transfer_read_models: Box::new(SqliteTransferReadModels::new(database.clone())),
            outbox: Box::new(SqliteOutbox::new(database.clone())),
            database,
            blockchain: external_systems.blockchain,
            kyc: external_systems.kyc,
            notifier: external_systems.notifier,
        }
    }
}

#[async_trait]
impl TransactionalPorts for Ports {
    /// The same ports, with the repository, the read models and the outbox writing in a new transaction.
    async fn begin(&self) -> Result<Self, Error> {
        let transaction = self.database.begin().await?;

        let external_systems = ExternalSystems {
            blockchain: Arc::clone(&self.blockchain),
            kyc: Arc::clone(&self.kyc),
            notifier: Arc::clone(&self.notifier),
        };

        Ok(Ports::new(transaction, external_systems))
    }

    async fn commit(self) -> Result<(), Error> {
        self.database.commit().await
    }

    fn outbox(&self) -> &dyn Outbox<Self> {
        self.outbox.as_ref()
    }
}
