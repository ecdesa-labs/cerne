use crate::application::read_models::pending_transfers::{PendingTransfer, PendingTransfers};
use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::application::Query;
use cerne::domain::ValueObject;
use cerne::sqlite::column;
use cerne::{Error, async_trait};
use serde::Deserialize;

/// Actor: the recipient. This is how bob finds the tx_hash of a transfer alice sent him.
#[derive(Deserialize)]
pub struct PendingTransfersQuery {
    pub recipient: Address,
}

#[async_trait]
impl Query<Ports> for PendingTransfersQuery {
    type ReadModel = PendingTransfers;

    async fn execute(&self, ports: &Ports) -> Result<PendingTransfers, Error> {
        // --- Ports -----------------------------------------------------------

        let select = sqlx::query(
            "SELECT tx_hash, sender, amount FROM transfers
             WHERE recipient = $1 AND status = 'Pending'
             ORDER BY sender, nonce",
        )
        .bind(self.recipient.to_string());

        let rows = ports.database.fetch_all(select).await?;

        // --- Read model ------------------------------------------------------

        let transfers = rows
            .iter()
            .map(|row| {
                Ok(PendingTransfer {
                    tx_hash: TxHash::new(column(row, "tx_hash")?)?,
                    sender: Address::new(column(row, "sender")?)?,
                    amount: column(row, "amount")?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let pending_transfers = PendingTransfers {
            recipient: self.recipient.clone(),
            transfers,
        };

        Ok(pending_transfers)
    }
}
