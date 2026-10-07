use crate::domain::entities::transfer::TransferStatus;
use crate::domain::events::transfer_canceled_by_sender::TransferCanceledBySender;
use crate::ports::Ports;
use cerne::application::{Command, Executed};
use cerne::domain::{BusinessRule, BusinessRules};
use cerne::{Error, async_trait};

/// Actor: the sender.
pub struct CancelTransferCommand {
    pub tx_hash: String,
    pub sender: String,
}

#[async_trait]
impl Command<Ports> for CancelTransferCommand {
    type Output = ();

    async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
        // --- Ports -----------------------------------------------------------

        let transfer = ports.transfers.load(&self.tx_hash).await?;

        // --- Business rules --------------------------------------------------

        let actor_is_the_sender = transfer.sender == self.sender;
        let is_still_pending = transfer.status == TransferStatus::Pending;

        BusinessRules::new(vec![
            BusinessRule::new("only the sender cancels", move || actor_is_the_sender),
            BusinessRule::new("transfer is still pending", move || is_still_pending),
        ])
        .check()?;

        // --- Aggregate -------------------------------------------------------

        let transfer = transfer.cancel()?;

        ports.transfers.save(transfer).await?;

        // --- Domain events ---------------------------------------------------

        let transfer_canceled = TransferCanceledBySender {
            tx_hash: self.tx_hash.clone(),
        };

        Ok(Executed {
            output: (),
            events: vec![Box::new(transfer_canceled)],
        })
    }
}
