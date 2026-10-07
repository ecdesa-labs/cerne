use crate::application::commands::notify_recipient::NotifyRecipientCommand;
use crate::domain::services::fees::Fees;
use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, Policy};

pub struct TransferCreated {
    pub tx_hash: String,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub fees: Fees,
}

impl DomainEvent<Ports> for TransferCreated {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        // --- Policies --------------------------------------------------------

        let tx_hash = self.tx_hash.clone();
        let sender = self.sender.clone();
        let recipient = self.recipient.clone();
        let amount = self.amount;

        let notify_recipient_policy = Policy::new(
            "whenever a transfer is created, notify the recipient",
            || true,
            move || {
                Box::new(NotifyRecipientCommand {
                    tx_hash: tx_hash.clone(),
                    sender: sender.clone(),
                    recipient: recipient.clone(),
                    amount,
                })
            },
        );

        Ok(Policies::new(vec![notify_recipient_policy]).trigger())
    }
}
