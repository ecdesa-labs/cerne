use crate::domain::entities::transfer::{Transfer, TransferStatus};
use crate::domain::events::failed_transaction_chained::FailedTransactionChained;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::application::{Command, Executed};
use cerne::domain::{BusinessRule, BusinessRules};
use cerne::{Error, async_trait};
use serde::{Deserialize, Serialize};

/// No actor: the policies "whenever a transfer is rejected (or canceled), chain it as failed" send this command.
/// The transaction goes in a block without moving RDEC, so the nonce of the sender moves on and MetaMask stops
/// waiting for it.
#[derive(Serialize, Deserialize)]
pub struct ChainFailedTransferCommand {
    pub tx_hash: TxHash,
}

impl ChainFailedTransferCommand {
    /// Sync and without ports, so it goes with the command wherever the command goes (D37).
    pub fn business_rules(&self, transfer: &Transfer) -> BusinessRules {
        let was_rejected_or_canceled = matches!(
            transfer.status,
            TransferStatus::Rejected | TransferStatus::Canceled
        );
        let not_chained_yet = !transfer.chained;

        BusinessRules::new(vec![
            BusinessRule::new("transfer was rejected or canceled", move || {
                was_rejected_or_canceled
            }),
            BusinessRule::new("transfer is not chained yet", move || not_chained_yet),
        ])
    }
}

#[async_trait]
impl Command<Ports> for ChainFailedTransferCommand {
    type Output = ();

    async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
        // --- Ports -----------------------------------------------------------

        let transfer = ports.transfers.load(&self.tx_hash).await?;

        // --- Business rules --------------------------------------------------

        self.business_rules(&transfer).check()?;

        // --- External system: Blockchain -------------------------------------

        let tx_hash = ports
            .blockchain
            .send_failed_transaction(&transfer.signed_transaction)
            .await?;

        // --- Aggregate -------------------------------------------------------

        let transfer = transfer.chain()?;

        ports.transfers.save(transfer).await?;

        // --- Domain events ---------------------------------------------------

        let failed_transaction_chained = FailedTransactionChained { tx_hash };

        Ok(Executed {
            output: (),
            events: vec![Box::new(failed_transaction_chained)],
        })
    }
}
