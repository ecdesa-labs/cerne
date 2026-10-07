use crate::domain::services::fees::Fees;
use crate::domain::value_objects::tx_hash::TxHash;
use cerne::{Error, async_trait};

/// External system "Blockchain": holds the RDEC balances and records the transactions.
#[async_trait]
pub trait Blockchain: Send + Sync {
    async fn available_rdec(&self, wallet: &str) -> Result<u64, Error>;

    /// The nonce the next transaction of `wallet` must carry: how many it has sent so far.
    async fn next_nonce(&self, wallet: &str) -> Result<u64, Error>;

    /// Sends the transaction (`sender` pays `amount` + the `fees`, `recipient` receives `amount`) and returns its hash.
    async fn send_transaction(
        &self,
        sender: &str,
        recipient: &str,
        amount: u64,
        fees: Fees,
        nonce: u64,
    ) -> Result<TxHash, Error>;
}
