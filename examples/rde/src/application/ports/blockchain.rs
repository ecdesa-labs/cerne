use crate::domain::services::fees::Fees;
use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::signed_transaction::SignedTransaction;
use crate::domain::value_objects::tx_hash::TxHash;
use cerne::{Error, async_trait};

/// External system "Blockchain": holds the RDEC balances, the nonces and the blocks. Every amount is in wei.
///
/// It is also what MetaMask reads through the rde (`eth_getBalance`, `eth_blockNumber`, `eth_getTransactionReceipt`).
#[async_trait]
pub trait Blockchain: Send + Sync {
    async fn available_rdec(&self, wallet: &Address) -> Result<u128, Error>;

    /// The nonce the next transaction of `wallet` must carry: how many of its transactions are in a block.
    async fn next_nonce(&self, wallet: &Address) -> Result<u64, Error>;

    /// What the chain charges per unit of gas, in wei.
    async fn gas_price(&self) -> Result<u128, Error>;

    /// Puts the transaction in a new block: the sender pays the amount and the `fees`, the recipient receives the
    /// amount. Returns its hash.
    async fn send_transaction(
        &self,
        signed_transaction: &SignedTransaction,
        fees: Fees,
    ) -> Result<TxHash, Error>;

    /// Puts the transaction in a new block as failed: no balance changes, only the nonce of the sender moves on.
    async fn send_failed_transaction(
        &self,
        signed_transaction: &SignedTransaction,
    ) -> Result<TxHash, Error>;

    /// The number of the last block.
    async fn block_number(&self) -> Result<u64, Error>;

    async fn block(&self, number: u64) -> Result<Option<Block>, Error>;

    /// `None` while the transaction is in no block.
    async fn receipt(&self, tx_hash: &TxHash) -> Result<Option<Receipt>, Error>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub number: u64,
    pub hash: String,
    pub parent_hash: String,
    /// Seconds since 1970.
    pub timestamp: u64,
    /// In wei per unit of gas.
    pub base_fee_per_gas: u128,
    pub transactions: Vec<TxHash>,
}

/// What happened to a transaction in a block.
#[derive(Debug, Clone, PartialEq)]
pub struct Receipt {
    pub tx_hash: TxHash,
    pub block_number: u64,
    pub block_hash: String,
    pub sender: Address,
    pub recipient: Address,
    /// `false` for a rejected or canceled transfer: in the block, but without moving RDEC.
    pub succeeded: bool,
    pub gas_used: u64,
    /// In wei per unit of gas.
    pub gas_price: u128,
}
