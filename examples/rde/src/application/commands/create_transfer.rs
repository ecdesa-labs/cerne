use crate::application::queries::open_transfers::OpenTransfersQuery;
use crate::domain::entities::transfer::{Transfer, TransferProps};
use crate::domain::events::transfer_created::TransferCreated;
use crate::domain::services::fees::estimate_fees;
use crate::domain::value_objects::signed_transaction::SignedTransaction;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::application::{Command, Executed, Query};
use cerne::domain::{BusinessRule, BusinessRules, Entity};
use cerne::{Error, async_trait};
use serde::Deserialize;

/// Actor: the sender, through MetaMask (`eth_sendRawTransaction`). The only field is the transaction MetaMask signed:
/// who sends, to whom and how much come out of it. There is no id: the transfer is the hash of that transaction.
#[derive(Deserialize)]
pub struct CreateTransferCommand {
    pub signed_transaction: SignedTransaction,
}

#[async_trait]
impl Command<Ports> for CreateTransferCommand {
    type Output = TxHash; // the id of the new transfer, which MetaMask follows

    async fn execute(&self, ports: &Ports) -> Result<Executed<TxHash, Ports>, Error> {
        // --- Domain service --------------------------------------------------

        let fees = estimate_fees(&self.signed_transaction);
        let cost = self.signed_transaction.amount() + fees.total();

        // --- Ports -----------------------------------------------------------

        let sender = self.signed_transaction.sender();
        let recipient = self.signed_transaction.recipient();

        let available_rdec = ports.blockchain.available_rdec(sender).await?;
        let next_nonce = ports.blockchain.next_nonce(sender).await?;
        let sender_has_kyc = ports.kyc.is_verified(sender).await?;
        let recipient_has_kyc = ports.kyc.is_verified(recipient).await?;

        let open_transfers_query = OpenTransfersQuery {
            sender: sender.clone(),
        };

        let open_transfers = open_transfers_query.execute(ports).await?;
        let sender_has_an_open_transfer = !open_transfers.transfers.is_empty();

        // --- Business rules --------------------------------------------------

        let sender_can_pay = available_rdec >= cost;
        let nonce_is_the_next_one = self.signed_transaction.nonce() == next_nonce;

        BusinessRules::new(vec![
            BusinessRule::new("sender has RDEC available", move || sender_can_pay),
            BusinessRule::new("nonce is the next one of the sender", move || {
                nonce_is_the_next_one
            }),
            BusinessRule::new("sender has no open transfer", move || {
                !sender_has_an_open_transfer
            }),
            BusinessRule::new("sender has KYC", move || sender_has_kyc),
            BusinessRule::new("recipient has KYC", move || recipient_has_kyc),
        ])
        .check()?;

        // --- Aggregate -------------------------------------------------------

        let transfer = Transfer::new(TransferProps {
            signed_transaction: self.signed_transaction.clone(),
            fees,
        })?;

        let tx_hash = ports.transfers.save(transfer).await?;

        // --- Domain events ---------------------------------------------------

        let transfer_created = TransferCreated {
            tx_hash: tx_hash.clone(),
            sender: sender.clone(),
            recipient: recipient.clone(),
            amount: self.signed_transaction.amount(),
            fees,
        };

        Ok(Executed {
            output: tx_hash,
            events: vec![Box::new(transfer_created)],
        })
    }
}
