use crate::application::read_models::pending_transfers::PendingTransfer;
use cerne::{Error, async_trait};

/// Reads the transfers already saved, shaped for the screens of the actors.
#[async_trait]
pub trait TransferReadModels: Send + Sync {
    /// The transfers still pending whose recipient is `recipient`, by sender and nonce.
    async fn pending_transfers(&self, recipient: &str) -> Result<Vec<PendingTransfer>, Error>;
}
