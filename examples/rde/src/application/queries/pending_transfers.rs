use crate::application::read_models::pending_transfers::PendingTransfers;
use crate::ports::Ports;
use cerne::application::Query;
use cerne::{Error, async_trait};
use serde::Deserialize;

/// Actor: the recipient. This is how bob finds the tx_hash of a transfer alice sent him.
#[derive(Deserialize)]
pub struct PendingTransfersQuery {
    pub recipient: String,
}

#[async_trait]
impl Query<Ports> for PendingTransfersQuery {
    type ReadModel = PendingTransfers;

    async fn execute(&self, ports: &Ports) -> Result<PendingTransfers, Error> {
        // --- Ports -----------------------------------------------------------

        let transfers = ports
            .transfer_read_models
            .pending_transfers(&self.recipient)
            .await?;

        // --- Read model ------------------------------------------------------

        let pending_transfers = PendingTransfers {
            recipient: self.recipient.clone(),
            transfers,
        };

        Ok(pending_transfers)
    }
}
