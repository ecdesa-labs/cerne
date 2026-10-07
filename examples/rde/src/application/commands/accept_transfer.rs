use crate::domain::entities::transfer::TransferStatus;
use crate::domain::events::transfer_accepted_by_recipient::TransferAcceptedByRecipient;
use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::application::{Command, Executed};
use cerne::domain::{BusinessRule, BusinessRules};
use cerne::{Error, async_trait};
use serde::Deserialize;

/// Actor: the recipient.
#[derive(Deserialize)]
pub struct AcceptTransferCommand {
    pub tx_hash: TxHash,
    pub recipient: Address,
}

#[async_trait]
impl Command<Ports> for AcceptTransferCommand {
    type Output = ();

    async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
        // --- Ports -----------------------------------------------------------

        let transfer = ports.transfers.load(&self.tx_hash).await?;

        // --- Business rules --------------------------------------------------

        let actor_is_the_recipient = transfer.recipient == self.recipient;
        let is_still_pending = transfer.status == TransferStatus::Pending;

        BusinessRules::new(vec![
            BusinessRule::new("only the recipient accepts", move || actor_is_the_recipient),
            BusinessRule::new("transfer is still pending", move || is_still_pending),
        ])
        .check()?;

        // --- Aggregate -------------------------------------------------------

        let transfer = transfer.accept()?;

        ports.transfers.save(transfer).await?;

        // --- Domain events ---------------------------------------------------

        let transfer_accepted = TransferAcceptedByRecipient {
            tx_hash: self.tx_hash.clone(),
        };

        Ok(Executed {
            output: (),
            events: vec![Box::new(transfer_accepted)],
        })
    }
}
