use crate::domain::events::recipient_notified::RecipientNotified;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::application::{Command, Executed};
use cerne::{Error, async_trait};
use serde::{Deserialize, Serialize};

/// No actor: the policy "whenever a transfer is created, notify the recipient" sends this command.
#[derive(Serialize, Deserialize)]
pub struct NotifyRecipientCommand {
    pub tx_hash: TxHash,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
}

#[async_trait]
impl Command<Ports> for NotifyRecipientCommand {
    type Output = ();

    async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
        // --- External system: Notifier ---------------------------------------

        let message = format!(
            "{} sent you {} RDEC. Accept or reject the transfer {}.",
            self.sender, self.amount, self.tx_hash
        );

        ports.notifier.notify(&self.recipient, &message).await?;

        // --- Domain events ---------------------------------------------------

        let recipient_notified = RecipientNotified {
            tx_hash: self.tx_hash.clone(),
            recipient: self.recipient.clone(),
        };

        Ok(Executed {
            output: (),
            events: vec![Box::new(recipient_notified)],
        })
    }
}
