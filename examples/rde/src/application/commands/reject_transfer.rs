use crate::domain::entities::transfer::TransferStatus;
use crate::domain::events::transfer_rejected_by_recipient::TransferRejectedByRecipient;
use crate::ports::Ports;
use cerne::application::{Command, Executed};
use cerne::domain::{BusinessRule, BusinessRules};
use cerne::{Error, async_trait};

/// Actor: the recipient.
pub struct RejectTransferCommand {
    pub tx_hash: String,
    pub recipient: String,
}

#[async_trait]
impl Command<Ports> for RejectTransferCommand {
    type Output = ();

    async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
        // --- Ports -----------------------------------------------------------

        let transfer = ports.transfers.load(&self.tx_hash).await?;

        // --- Business rules --------------------------------------------------

        let actor_is_the_recipient = transfer.recipient == self.recipient;
        let is_still_pending = transfer.status == TransferStatus::Pending;

        BusinessRules::new(vec![
            BusinessRule::new("only the recipient rejects", move || actor_is_the_recipient),
            BusinessRule::new("transfer is still pending", move || is_still_pending),
        ])
        .check()?;

        // --- Aggregate -------------------------------------------------------

        let transfer = transfer.reject()?;

        ports.transfers.save(transfer).await?;

        // --- Domain events ---------------------------------------------------

        let transfer_rejected = TransferRejectedByRecipient {
            tx_hash: self.tx_hash.clone(),
        };

        Ok(Executed {
            output: (),
            events: vec![Box::new(transfer_rejected)],
        })
    }
}
