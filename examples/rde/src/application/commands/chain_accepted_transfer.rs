use crate::domain::entities::transfer::TransferStatus;
use crate::domain::events::transaction_chained::TransactionChained;
use crate::ports::Ports;
use cerne::application::{Command, Executed};
use cerne::domain::{BusinessRule, BusinessRules};
use cerne::{Error, async_trait};

/// No actor: the policy "whenever a transfer is accepted, chain it" sends this command.
pub struct ChainAcceptedTransferCommand {
    pub tx_hash: String,
}

#[async_trait]
impl Command<Ports> for ChainAcceptedTransferCommand {
    type Output = ();

    async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
        // --- Ports -----------------------------------------------------------

        let transfer = ports.transfers.load(&self.tx_hash).await?;

        // --- Business rules --------------------------------------------------

        let was_accepted = transfer.status == TransferStatus::Accepted;
        let not_chained_yet = !transfer.chained;

        BusinessRules::new(vec![
            BusinessRule::new("transfer was accepted", move || was_accepted),
            BusinessRule::new("transfer is not chained yet", move || not_chained_yet),
        ])
        .check()?;

        // --- External system: Blockchain -------------------------------------

        let tx_hash = ports
            .blockchain
            .send_transaction(
                &transfer.sender,
                &transfer.recipient,
                transfer.amount,
                transfer.fees,
                transfer.nonce,
            )
            .await?;

        // --- Aggregate -------------------------------------------------------

        let transfer = transfer.chain()?;

        ports.transfers.save(transfer).await?;

        // --- Domain events ---------------------------------------------------

        let transaction_chained = TransactionChained { tx_hash };

        Ok(Executed {
            output: (),
            events: vec![Box::new(transaction_chained)],
        })
    }
}
