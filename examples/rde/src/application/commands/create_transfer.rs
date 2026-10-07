use crate::domain::entities::transfer::{Transfer, TransferProps};
use crate::domain::events::transfer_created::TransferCreated;
use crate::domain::services::fees::estimate_fees;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::application::{Command, Executed};
use cerne::domain::{BusinessRule, BusinessRules, Entity};
use cerne::{Error, async_trait};
use serde::Deserialize;

/// Actor: the sender. There is no id: the transfer is identified by the hash of its transaction.
#[derive(Deserialize)]
pub struct CreateTransferCommand {
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
}

#[async_trait]
impl Command<Ports> for CreateTransferCommand {
    type Output = TxHash; // the id of the new transfer

    async fn execute(&self, ports: &Ports) -> Result<Executed<TxHash, Ports>, Error> {
        // --- Domain service --------------------------------------------------

        let fees = estimate_fees(self.amount);
        let cost = self.amount + fees.total();

        // --- Ports -----------------------------------------------------------

        let nonce = ports.blockchain.next_nonce(&self.sender).await?;
        let available_rdec = ports.blockchain.available_rdec(&self.sender).await?;
        let sender_has_kyc = ports.kyc.is_verified(&self.sender).await?;
        let recipient_has_kyc = ports.kyc.is_verified(&self.recipient).await?;

        // --- Business rules --------------------------------------------------

        let sender_can_pay = available_rdec >= cost;

        BusinessRules::new(vec![
            BusinessRule::new("sender has RDEC available", move || sender_can_pay),
            BusinessRule::new("sender has KYC", move || sender_has_kyc),
            BusinessRule::new("recipient has KYC", move || recipient_has_kyc),
        ])
        .check()?;

        // --- Aggregate -------------------------------------------------------

        let transfer = Transfer::new(TransferProps {
            sender: self.sender.clone(),
            recipient: self.recipient.clone(),
            amount: self.amount,
            fees,
            nonce,
        })?;

        let tx_hash = ports.transfers.save(transfer).await?;

        // --- Domain events ---------------------------------------------------

        let transfer_created = TransferCreated {
            tx_hash: tx_hash.clone(),
            sender: self.sender.clone(),
            recipient: self.recipient.clone(),
            amount: self.amount,
            fees,
        };

        Ok(Executed {
            output: tx_hash,
            events: vec![Box::new(transfer_created)],
        })
    }
}
