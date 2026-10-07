use crate::application::ports::blockchain::Blockchain;
use crate::domain::services::fees::Fees;
use crate::domain::services::transaction_hash::transaction_hash;
use crate::domain::value_objects::tx_hash::TxHash;
use cerne::domain::ValueObject;
use cerne::{Error, InfrastructureError, async_trait};
use std::collections::HashMap;
use std::sync::Mutex;

/// A stand-in for the RDE chain: balances and nonces in `HashMap`s.
/// Like a real node, it refuses a reused nonce and hashes the transaction it receives.
pub struct InMemoryBlockchain {
    balances: Mutex<HashMap<String, u64>>,
    nonces: Mutex<HashMap<String, u64>>,
}

impl InMemoryBlockchain {
    pub fn new(balances: Vec<(&str, u64)>) -> Self {
        Self {
            balances: Mutex::new(
                balances
                    .into_iter()
                    .map(|(w, b)| (w.to_string(), b))
                    .collect(),
            ),
            nonces: Mutex::new(HashMap::new()),
        }
    }
}

fn refused(reason: String) -> Error {
    InfrastructureError::from(anyhow::anyhow!("chain refused: {reason}")).into()
}

#[async_trait]
impl Blockchain for InMemoryBlockchain {
    async fn available_rdec(&self, wallet: &str) -> Result<u64, Error> {
        Ok(*self.balances.lock().unwrap().get(wallet).unwrap_or(&0))
    }

    async fn next_nonce(&self, wallet: &str) -> Result<u64, Error> {
        Ok(*self.nonces.lock().unwrap().get(wallet).unwrap_or(&0))
    }

    async fn send_transaction(
        &self,
        sender: &str,
        recipient: &str,
        amount: u64,
        fees: Fees,
        nonce: u64,
    ) -> Result<TxHash, Error> {
        let mut balances = self.balances.lock().unwrap();
        let mut nonces = self.nonces.lock().unwrap();

        // --- Nonce: each transaction of a wallet carries the next one --------

        let expected_nonce = *nonces.get(sender).unwrap_or(&0);
        if nonce != expected_nonce {
            return Err(refused(format!(
                "{sender} expected nonce {expected_nonce}, got {nonce}"
            )));
        }

        // --- Balance: the sender pays the amount plus the fees ---------------

        let available = *balances.get(sender).unwrap_or(&0);
        let remaining = available
            .checked_sub(amount + fees.total())
            .ok_or_else(|| refused(format!("{sender} has only {available} RDEC")))?;

        // --- Record: move the RDEC and advance the nonce ---------------------

        balances.insert(sender.to_string(), remaining);
        *balances.entry(recipient.to_string()).or_insert(0) += amount;
        nonces.insert(sender.to_string(), nonce + 1);

        // --- Hash: like a node, recomputed from the transaction it received ----

        let tx_hash = TxHash::new(transaction_hash(sender, recipient, amount, fees, nonce))?;

        Ok(tx_hash)
    }
}
