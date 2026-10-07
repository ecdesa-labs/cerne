use crate::domain::entities::transfer::{Transfer, TransferStatus};
use crate::domain::services::fees::Fees;
use crate::domain::value_objects::signed_transaction::SignedTransaction;
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
                (tx_hash, signed_transaction, sender, recipient, amount, decarbonization_fee, gas_fee, nonce, status,
                 chained, cancellation, cancellation_tx_hash)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             ON CONFLICT (tx_hash) DO UPDATE SET
                status = excluded.status,
                chained = excluded.chained,
                cancellation = excluded.cancellation,
                cancellation_tx_hash = excluded.cancellation_tx_hash",
        )
        .bind(transfer.tx_hash.to_string())
        .bind(transfer.signed_transaction.to_string())
        .bind(transfer.sender.to_string())
        .bind(transfer.recipient.to_string())
        .bind(transfer.amount.to_string())
        .bind(transfer.fees.decarbonization.to_string())
        .bind(transfer.fees.gas.to_string())
        .bind(transfer.nonce as i64)
        .bind(status_name(transfer.status))
        .bind(transfer.chained)
        .bind(transfer.cancellation.as_ref().map(ToString::to_string))
        .bind(
            transfer
                .cancellation
                .as_ref()
                .map(|cancellation| cancellation.tx_hash().to_string()),
        );

        self.database.execute(upsert).await?;

        Ok(transfer.tx_hash)
    }
}

/// A row back into a transfer, through `validate`: a row that breaks an invariant is an error, not a transfer.
///
/// Who sends, to whom and how much come from the signed transaction, read again; the other columns with them are
/// there for the queries.
fn transfer_from(row: &SqliteRow) -> Result<Transfer, Error> {
    let signed_transaction = SignedTransaction::new(column(row, "signed_transaction")?)?;
    let cancellation = column::<Option<String>>(row, "cancellation")?
        .map(SignedTransaction::new)
        .transpose()?;

    let transfer = Transfer {
        tx_hash: TxHash::new(column(row, "tx_hash")?)?,
        sender: signed_transaction.sender().clone(),
        recipient: signed_transaction.recipient().clone(),
        amount: signed_transaction.amount(),
        nonce: signed_transaction.nonce(),
        signed_transaction,
        fees: Fees {
            decarbonization: wei(row, "decarbonization_fee")?,
            gas: wei(row, "gas_fee")?,
        },
        status: status_from(&column::<String>(row, "status")?)?,
        chained: column(row, "chained")?,
        cancellation,
    };

    Ok(transfer.validate()?)
}

/// An amount stored as decimal text.
fn wei(row: &SqliteRow, name: &str) -> Result<u128, Error> {
    let text: String = column(row, name)?;

    text.parse().map_err(|error| {
        InfrastructureError::from(anyhow::anyhow!("{name} is not wei: {error}")).into()
    })
}

fn status_name(status: TransferStatus) -> &'static str {
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
