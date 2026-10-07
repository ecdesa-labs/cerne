use crate::domain::events::cancellation_chained::CancellationChained;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::application::{Command, Executed};
use cerne::domain::{BusinessRule, BusinessRules};
use cerne::{ApplicationError, Error, async_trait};
use serde::{Deserialize, Serialize};

/// No actor: the policy "whenever a transfer is replaced by a cancellation, chain the cancellation" sends this
/// command. The cancellation goes in a block as succeeded, so MetaMask confirms it and shows the transfer as failed.
#[derive(Serialize, Deserialize)]
pub struct ChainCancellationCommand {
    pub tx_hash: TxHash,
}

#[async_trait]
impl Command<Ports> for ChainCancellationCommand {
    type Output = ();

    async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
        // --- Ports -----------------------------------------------------------

        let transfer = ports.transfers.load(&self.tx_hash).await?;

        let cancellation = transfer
            .cancellation
            .clone()
            .ok_or(ApplicationError::NotFound("cancellation"))?;

        // --- Business rules --------------------------------------------------

        let not_chained_yet = !transfer.chained;

        BusinessRules::new(vec![BusinessRule::new(
            "transfer is not chained yet",
            move || not_chained_yet,
        )])
        .check()?;

        // --- External system: Blockchain -------------------------------------

        let cancellation_tx_hash = ports.blockchain.send_cancellation(&cancellation).await?;

        // --- Aggregate -------------------------------------------------------

        let transfer = transfer.chain()?;

        ports.transfers.save(transfer).await?;

        // --- Domain events ---------------------------------------------------

        let cancellation_chained = CancellationChained {
            tx_hash: cancellation_tx_hash,
        };

        Ok(Executed {
            output: (),
            events: vec![Box::new(cancellation_chained)],
        })
    }
}
