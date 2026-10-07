use crate::application::read_models::pending_transfers::{PendingTransfer, PendingTransfers};
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
    pub recipient: String,
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
        .bind(&self.recipient);

        let rows = ports.database.fetch_all(select).await?;

        // --- Read model ------------------------------------------------------

        let transfers = rows
            .iter()
            .map(|row| {
                Ok(PendingTransfer {
                    tx_hash: TxHash::new(column(row, "tx_hash")?)?,
                    sender: column(row, "sender")?,
                    amount: column::<i64>(row, "amount")? as u64,
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
