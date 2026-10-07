use crate::application::ports::blockchain::{Block, Blockchain, Receipt};
use crate::domain::services::fees::Fees;
use crate::domain::services::transaction_hash::transaction_hash;
use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::signed_transaction::SignedTransaction;
use crate::domain::value_objects::tx_hash::TxHash;
use cerne::domain::ValueObject;
use cerne::sqlite::{SqliteDatabase, column};
use cerne::{Error, InfrastructureError, async_trait};
use sqlx::sqlite::SqliteRow;
use std::time::{SystemTime, UNIX_EPOCH};

/// 1 gwei: the gas price and the base fee of every block.
const GAS_PRICE: u128 = 1_000_000_000;

/// The RDE chain of the server, in a SQLite database of its own (`chain_migrations/`): the same chain as the
/// `InMemoryBlockchain`, but one that can be read with `sqlite3` while the server runs. Like a real node, it refuses
/// a nonce out of order, and each transaction it receives goes in a block of its own.
///
/// Each write runs in a transaction of the chain database, never in the one of the transfers: the chain is an
/// external system (D34).
pub struct SqliteBlockchain {
    database: SqliteDatabase,
}

impl SqliteBlockchain {
    /// Creates the tables, the genesis block (number 0) and these balances, in wei, unless the chain already has
    /// them.
    pub async fn open(
        database: SqliteDatabase,
        balances: Vec<(Address, u128)>,
    ) -> Result<Self, Error> {
        database
            .migrate(&sqlx::migrate!("./chain_migrations"))
            .await?;

        let blockchain = Self { database };

        if blockchain.block(0).await?.is_none() {
            let transaction = blockchain.database.begin().await?;

            for (wallet, wei) in balances {
                set_balance(&transaction, &wallet, wei).await?;
            }

            add_block(&transaction, None).await?;
            transaction.commit().await?;
        }

        Ok(blockchain)
    }
}

fn refused(reason: String) -> Error {
    InfrastructureError::from(anyhow::anyhow!("chain refused: {reason}")).into()
}

fn block_hash(number: u64) -> String {
    transaction_hash(format!("block {number}").as_bytes())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// An amount stored as decimal text.
fn wei(row: &SqliteRow, name: &str) -> Result<u128, Error> {
    let text: String = column(row, name)?;

    text.parse()
        .map_err(|error| refused(format!("{name} is not wei: {error}")))
}

// --- Reads and writes, in the database given (the pool or a transaction) -----

async fn balance(database: &SqliteDatabase, wallet: &Address) -> Result<u128, Error> {
    let select =
        sqlx::query("SELECT wei FROM chain_balances WHERE wallet = $1").bind(wallet.to_string());

    match database.fetch_optional(select).await? {
        Some(row) => wei(&row, "wei"),
        None => Ok(0),
    }
}

async fn set_balance(database: &SqliteDatabase, wallet: &Address, wei: u128) -> Result<(), Error> {
    let upsert = sqlx::query(
        "INSERT INTO chain_balances (wallet, wei) VALUES ($1, $2)
         ON CONFLICT (wallet) DO UPDATE SET wei = excluded.wei",
    )
    .bind(wallet.to_string())
    .bind(wei.to_string());

    database.execute(upsert).await?;

    Ok(())
}

async fn next_nonce(database: &SqliteDatabase, wallet: &Address) -> Result<u64, Error> {
    let select = sqlx::query("SELECT next_nonce FROM chain_nonces WHERE wallet = $1")
        .bind(wallet.to_string());

    match database.fetch_optional(select).await? {
        Some(row) => Ok(column::<i64>(&row, "next_nonce")? as u64),
        None => Ok(0),
    }
}

/// The nonce of the transaction must be the next one of its sender, as on a real chain.
async fn check_nonce(
    database: &SqliteDatabase,
    signed_transaction: &SignedTransaction,
) -> Result<(), Error> {
    let sender = signed_transaction.sender();
    let expected_nonce = next_nonce(database, sender).await?;
    let nonce = signed_transaction.nonce();

    if nonce != expected_nonce {
        return Err(refused(format!(
            "{sender} expected nonce {expected_nonce}, got {nonce}"
        )));
    }

    Ok(())
}

/// A new block, with this transaction or none (the genesis block). Returns its number and hash.
async fn add_block(
    database: &SqliteDatabase,
    tx_hash: Option<&TxHash>,
) -> Result<(u64, String), Error> {
    let last = sqlx::query("SELECT number, hash FROM chain_blocks ORDER BY number DESC LIMIT 1");

    let (number, parent_hash) = match database.fetch_optional(last).await? {
        Some(row) => (
            column::<i64>(&row, "number")? as u64 + 1,
            column::<String>(&row, "hash")?,
        ),
        None => (0, block_hash(u64::MAX)),
    };

    let hash = block_hash(number);

    let insert = sqlx::query(
        "INSERT INTO chain_blocks (number, hash, parent_hash, timestamp, base_fee_per_gas, tx_hash)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(number as i64)
    .bind(&hash)
    .bind(parent_hash)
    .bind(now() as i64)
    .bind(GAS_PRICE.to_string())
    .bind(tx_hash.map(ToString::to_string));

    database.execute(insert).await?;

    Ok((number, hash))
}

/// A new block with this one transaction, its receipt, and the nonce of the sender moved on. `gas_used` is what the
/// receipt says the sender paid for: 0 when nothing was charged.
async fn add_transaction_block(
    database: &SqliteDatabase,
    signed_transaction: &SignedTransaction,
    succeeded: bool,
    gas_used: u64,
) -> Result<TxHash, Error> {
    let tx_hash = signed_transaction.tx_hash().clone();
    let sender = signed_transaction.sender();

    let (block_number, block_hash) = add_block(database, Some(&tx_hash)).await?;

    let insert_receipt = sqlx::query(
        "INSERT INTO chain_receipts
            (tx_hash, block_number, block_hash, sender, recipient, succeeded, gas_used, gas_price)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(tx_hash.to_string())
    .bind(block_number as i64)
    .bind(block_hash)
    .bind(sender.to_string())
    .bind(signed_transaction.recipient().to_string())
    .bind(succeeded)
    .bind(gas_used as i64)
    .bind(signed_transaction.max_fee_per_gas().to_string());

    database.execute(insert_receipt).await?;

    let upsert_nonce = sqlx::query(
        "INSERT INTO chain_nonces (wallet, next_nonce) VALUES ($1, $2)
         ON CONFLICT (wallet) DO UPDATE SET next_nonce = excluded.next_nonce",
    )
    .bind(sender.to_string())
    .bind(signed_transaction.nonce() as i64 + 1);

    database.execute(upsert_nonce).await?;

    Ok(tx_hash)
}

#[async_trait]
impl Blockchain for SqliteBlockchain {
    async fn available_rdec(&self, wallet: &Address) -> Result<u128, Error> {
        balance(&self.database, wallet).await
    }

    async fn next_nonce(&self, wallet: &Address) -> Result<u64, Error> {
        next_nonce(&self.database, wallet).await
    }

    async fn gas_price(&self) -> Result<u128, Error> {
        Ok(GAS_PRICE)
    }

    async fn send_transaction(
        &self,
        signed_transaction: &SignedTransaction,
        fees: Fees,
    ) -> Result<TxHash, Error> {
        let transaction = self.database.begin().await?;

        check_nonce(&transaction, signed_transaction).await?;

        // --- Balance: the sender pays the amount plus the fees ---------------

        let sender = signed_transaction.sender();
        let recipient = signed_transaction.recipient();
        let amount = signed_transaction.amount();

        let available = balance(&transaction, sender).await?;
        let remaining = available
            .checked_sub(amount + fees.total())
            .ok_or_else(|| refused(format!("{sender} has only {available} wei")))?;

        set_balance(&transaction, sender, remaining).await?;

        let recipient_balance = balance(&transaction, recipient).await?;

        set_balance(&transaction, recipient, recipient_balance + amount).await?;

        // --- Block -----------------------------------------------------------

        let gas_used = signed_transaction.gas_limit();
        let tx_hash =
            add_transaction_block(&transaction, signed_transaction, true, gas_used).await?;

        transaction.commit().await?;

        Ok(tx_hash)
    }

    async fn send_failed_transaction(
        &self,
        signed_transaction: &SignedTransaction,
    ) -> Result<TxHash, Error> {
        let transaction = self.database.begin().await?;

        check_nonce(&transaction, signed_transaction).await?;

        let tx_hash = add_transaction_block(&transaction, signed_transaction, false, 0).await?;

        transaction.commit().await?;

        Ok(tx_hash)
    }

    async fn send_cancellation(&self, cancellation: &SignedTransaction) -> Result<TxHash, Error> {
        let transaction = self.database.begin().await?;

        check_nonce(&transaction, cancellation).await?;

        let tx_hash = add_transaction_block(&transaction, cancellation, true, 0).await?;

        transaction.commit().await?;

        Ok(tx_hash)
    }

    async fn block_number(&self) -> Result<u64, Error> {
        let select = sqlx::query("SELECT MAX(number) AS number FROM chain_blocks");

        let row = self.database.fetch_one(select).await?;

        Ok(column::<i64>(&row, "number")? as u64)
    }

    async fn block(&self, number: u64) -> Result<Option<Block>, Error> {
        let select =
            sqlx::query("SELECT * FROM chain_blocks WHERE number = $1").bind(number as i64);

        let Some(row) = self.database.fetch_optional(select).await? else {
            return Ok(None);
        };

        let transactions = column::<Option<String>>(&row, "tx_hash")?
            .map(TxHash::new)
            .transpose()?
            .into_iter()
            .collect();

        Ok(Some(Block {
            number,
            hash: column(&row, "hash")?,
            parent_hash: column(&row, "parent_hash")?,
            timestamp: column::<i64>(&row, "timestamp")? as u64,
            base_fee_per_gas: wei(&row, "base_fee_per_gas")?,
            transactions,
        }))
    }

    async fn block_by_hash(&self, hash: &str) -> Result<Option<Block>, Error> {
        let select = sqlx::query("SELECT number FROM chain_blocks WHERE hash = $1").bind(hash);

        match self.database.fetch_optional(select).await? {
            Some(row) => self.block(column::<i64>(&row, "number")? as u64).await,
            None => Ok(None),
        }
    }

    async fn receipt(&self, tx_hash: &TxHash) -> Result<Option<Receipt>, Error> {
        let select = sqlx::query("SELECT * FROM chain_receipts WHERE tx_hash = $1")
            .bind(tx_hash.to_string());

        let Some(row) = self.database.fetch_optional(select).await? else {
            return Ok(None);
        };

        Ok(Some(Receipt {
            tx_hash: tx_hash.clone(),
            block_number: column::<i64>(&row, "block_number")? as u64,
            block_hash: column(&row, "block_hash")?,
            sender: Address::new(column(&row, "sender")?)?,
            recipient: Address::new(column(&row, "recipient")?)?,
            succeeded: column(&row, "succeeded")?,
            gas_used: column::<i64>(&row, "gas_used")? as u64,
            gas_price: wei(&row, "gas_price")?,
        }))
    }
}
