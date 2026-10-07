use crate::domain::entities::transfer::{Transfer, TransferStatus};
use crate::domain::services::fees::Fees;
use crate::domain::value_objects::tx_hash::TxHash;
use cerne::application::Repository;
use cerne::domain::{Entity, ValueObject};
use cerne::sqlite::{SqliteDatabase, column};
use cerne::{ApplicationError, Error, InfrastructureError, async_trait};
use sqlx::sqlite::SqliteRow;

/// The transfers in the table `transfers` (`migrations/0001_create_transfers.sql`).
///
/// The SQL also runs on Postgres: moving there is swapping `SqliteDatabase` for `PostgresDatabase`.
pub struct SqliteTransferRepository {
    database: SqliteDatabase,
}

impl SqliteTransferRepository {
    pub fn new(database: SqliteDatabase) -> Self {
        Self { database }
    }
}

#[async_trait]
impl Repository<Transfer> for SqliteTransferRepository {
    async fn load(&self, tx_hash: &TxHash) -> Result<Transfer, Error> {
        let select =
            sqlx::query("SELECT * FROM transfers WHERE tx_hash = $1").bind(tx_hash.to_string());

        let row = self
            .database
            .fetch_optional(select)
            .await?
            .ok_or(ApplicationError::NotFound("transfer"))?;

        transfer_from(&row)
    }

    /// The tx_hash comes from the content of the transfer, so it is always there: insert or update by it.
    async fn save(&self, transfer: Transfer) -> Result<TxHash, Error> {
        let upsert = sqlx::query(
            "INSERT INTO transfers
                (tx_hash, sender, recipient, amount, decarbonization_fee, gas_fee, nonce, status, chained)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (tx_hash) DO UPDATE SET status = excluded.status, chained = excluded.chained",
        )
        .bind(transfer.tx_hash.to_string())
        .bind(&transfer.sender)
        .bind(&transfer.recipient)
        .bind(transfer.amount as i64)
        .bind(transfer.fees.decarbonization as i64)
        .bind(transfer.fees.gas as i64)
        .bind(transfer.nonce as i64)
        .bind(status_name(transfer.status))
        .bind(transfer.chained);

        self.database.execute(upsert).await?;

        Ok(transfer.tx_hash)
    }
}

/// A row back into a transfer, through `validate`: a row that breaks an invariant is an error, not a transfer.
fn transfer_from(row: &SqliteRow) -> Result<Transfer, Error> {
    let transfer = Transfer {
        tx_hash: TxHash::new(column(row, "tx_hash")?)?,
        sender: column(row, "sender")?,
        recipient: column(row, "recipient")?,
        amount: column::<i64>(row, "amount")? as u64,
        fees: Fees {
            decarbonization: column::<i64>(row, "decarbonization_fee")? as u64,
            gas: column::<i64>(row, "gas_fee")? as u64,
        },
        nonce: column::<i64>(row, "nonce")? as u64,
        status: status_from(&column::<String>(row, "status")?)?,
        chained: column(row, "chained")?,
    };

    Ok(transfer.validate()?)
}

pub(crate) fn status_name(status: TransferStatus) -> &'static str {
    match status {
        TransferStatus::Pending => "Pending",
        TransferStatus::Accepted => "Accepted",
        TransferStatus::Rejected => "Rejected",
        TransferStatus::Canceled => "Canceled",
    }
}

fn status_from(name: &str) -> Result<TransferStatus, Error> {
    match name {
        "Pending" => Ok(TransferStatus::Pending),
        "Accepted" => Ok(TransferStatus::Accepted),
        "Rejected" => Ok(TransferStatus::Rejected),
        "Canceled" => Ok(TransferStatus::Canceled),
        _ => Err(InfrastructureError::from(anyhow::anyhow!(
            "unknown transfer status {name}"
        )))?,
    }
}
