use crate::application::queries::open_transfers::OpenTransfersQuery;
use crate::domain::entities::transfer::TransferStatus;
use crate::domain::events::transfer_replaced_by_cancellation::TransferReplacedByCancellation;
use crate::domain::value_objects::signed_transaction::SignedTransaction;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::application::{Command, Executed, Query};
use cerne::domain::{BusinessRule, BusinessRules};
use cerne::{ApplicationError, Error, async_trait};
use serde::Deserialize;

/// Actor: the sender, clicking "Cancel" in MetaMask (`eth_sendRawTransaction`). MetaMask signs a new transaction with
/// the nonce of the pending transfer, nothing to the sender itself (`docs/METAMASK.md`). It replaces the transfer:
/// the cancellation goes to the chain, and the transfer never does.
#[derive(Deserialize)]
pub struct SendCancellationCommand {
    pub cancellation: SignedTransaction,
}

#[async_trait]
impl Command<Ports> for SendCancellationCommand {
    type Output = TxHash; // the hash of the cancellation, which MetaMask follows from now on

    async fn execute(&self, ports: &Ports) -> Result<Executed<TxHash, Ports>, Error> {
        // --- Ports -----------------------------------------------------------

        let open_transfers_query = OpenTransfersQuery {
            sender: self.cancellation.sender().clone(),
        };

        let open_transfers = open_transfers_query.execute(ports).await?;

        let open_transfer = open_transfers
            .transfers
            .first()
            .ok_or(ApplicationError::NotFound("open transfer"))?;

        let transfer = ports.transfers.load(open_transfer).await?;

        // --- Business rules --------------------------------------------------

        // Pending is what is in no block proposal yet: accepted or rejected, the transfer is on its way to the chain.
        let is_still_pending = transfer.status == TransferStatus::Pending;
        let cancellation_has_the_nonce_of_the_transfer =
            self.cancellation.nonce() == transfer.nonce;

        BusinessRules::new(vec![
            BusinessRule::new("transfer is still pending", move || is_still_pending),
            BusinessRule::new("cancellation has the nonce of the transfer", move || {
                cancellation_has_the_nonce_of_the_transfer
            }),
        ])
        .check()?;

        // --- Aggregate -------------------------------------------------------

        let transfer = transfer.replace_by(self.cancellation.clone())?;

        let tx_hash = ports.transfers.save(transfer).await?;

        // --- Domain events ---------------------------------------------------

        let transfer_replaced_by_cancellation = TransferReplacedByCancellation {
            tx_hash,
            cancellation_tx_hash: self.cancellation.tx_hash().clone(),
        };

        Ok(Executed {
            output: self.cancellation.tx_hash().clone(),
            events: vec![Box::new(transfer_replaced_by_cancellation)],
        })
    }
}
