use crate::application::ports::transfer_read_models::TransferReadModels;
use crate::application::read_models::pending_transfers::PendingTransfer;
use crate::domain::entities::transfer::TransferStatus;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::infrastructure::sqlite_transfer_repository::status_name;
use cerne::domain::ValueObject;
use cerne::sqlite::{SqliteDatabase, column};
use cerne::{Error, async_trait};

/// Reads the table `transfers` with a `SELECT` shaped like each screen: only the columns the read model shows.
pub struct SqliteTransferReadModels {
    database: SqliteDatabase,
}

impl SqliteTransferReadModels {
    pub fn new(database: SqliteDatabase) -> Self {
        Self { database }
    }
}

#[async_trait]
impl TransferReadModels for SqliteTransferReadModels {
    async fn pending_transfers(&self, recipient: &str) -> Result<Vec<PendingTransfer>, Error> {
        let select = sqlx::query(
            "SELECT tx_hash, sender, amount FROM transfers
             WHERE recipient = $1 AND status = $2
             ORDER BY sender, nonce",
        )
        .bind(recipient)
        .bind(status_name(TransferStatus::Pending));

        let rows = self.database.fetch_all(select).await?;

        rows.iter()
            .map(|row| {
                Ok(PendingTransfer {
                    tx_hash: TxHash::new(column(row, "tx_hash")?)?,
                    sender: column(row, "sender")?,
                    amount: column::<i64>(row, "amount")? as u64,
                })
            })
            .collect()
    }
}
