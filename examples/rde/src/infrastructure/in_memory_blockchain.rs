use crate::application::ports::blockchain::{Block, Blockchain, Receipt};
use crate::domain::services::fees::Fees;
use crate::domain::services::transaction_hash::transaction_hash;
use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::signed_transaction::SignedTransaction;
use crate::domain::value_objects::tx_hash::TxHash;
use cerne::{Error, InfrastructureError, async_trait};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// 1 gwei: the gas price and the base fee of every block.
const GAS_PRICE: u128 = 1_000_000_000;

/// A stand-in for the RDE chain: balances, nonces, blocks and receipts in memory.
/// Like a real node, it refuses a nonce out of order, and each transaction it receives goes in a block of its own.
pub struct InMemoryBlockchain {
    state: Mutex<State>,
}

struct State {
    balances: HashMap<Address, u128>,
    nonces: HashMap<Address, u64>,
    blocks: Vec<Block>,
    receipts: HashMap<TxHash, Receipt>,
}

impl InMemoryBlockchain {
    /// The chain starts with the genesis block (number 0) and these balances, in wei.
    pub fn new(balances: Vec<(Address, u128)>) -> Self {
        let genesis = Block {
            number: 0,
            hash: block_hash(0),
            parent_hash: block_hash(u64::MAX),
            timestamp: now(),
            base_fee_per_gas: GAS_PRICE,
            transactions: vec![],
        };

        Self {
            state: Mutex::new(State {
                balances: balances.into_iter().collect(),
                nonces: HashMap::new(),
                blocks: vec![genesis],
                receipts: HashMap::new(),
            }),
        }
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

impl State {
    fn next_nonce(&self, wallet: &Address) -> u64 {
        *self.nonces.get(wallet).unwrap_or(&0)
    }

    /// The nonce of the transaction must be the next one of its sender, as on a real chain.
    fn check_nonce(&self, signed_transaction: &SignedTransaction) -> Result<(), Error> {
        let sender = signed_transaction.sender();
        let expected_nonce = self.next_nonce(sender);
        let nonce = signed_transaction.nonce();

        if nonce != expected_nonce {
            return Err(refused(format!(
                "{sender} expected nonce {expected_nonce}, got {nonce}"
            )));
        }

        Ok(())
    }

    /// A new block with this one transaction, its receipt, and the nonce of the sender moved on. `gas_used` is what
    /// the receipt says the sender paid for: 0 when nothing was charged.
    fn add_block(
        &mut self,
        signed_transaction: &SignedTransaction,
        succeeded: bool,
        gas_used: u64,
    ) -> TxHash {
        let parent = self.blocks.last().map(|block| block.hash.clone());
        let number = self.blocks.len() as u64;
        let tx_hash = signed_transaction.tx_hash().clone();

        let block = Block {
            number,
            hash: block_hash(number),
            parent_hash: parent.unwrap_or_default(),
            timestamp: now(),
            base_fee_per_gas: GAS_PRICE,
            transactions: vec![tx_hash.clone()],
        };

        let receipt = Receipt {
            tx_hash: tx_hash.clone(),
            block_number: number,
            block_hash: block.hash.clone(),
            sender: signed_transaction.sender().clone(),
            recipient: signed_transaction.recipient().clone(),
            succeeded,
            gas_used,
            gas_price: signed_transaction.max_fee_per_gas(),
        };

        self.blocks.push(block);
        self.receipts.insert(tx_hash.clone(), receipt);
        self.nonces.insert(
            signed_transaction.sender().clone(),
            signed_transaction.nonce() + 1,
        );

        tx_hash
    }
}

#[async_trait]
impl Blockchain for InMemoryBlockchain {
    async fn available_rdec(&self, wallet: &Address) -> Result<u128, Error> {
        Ok(*self
            .state
            .lock()
            .unwrap()
            .balances
            .get(wallet)
            .unwrap_or(&0))
    }

    async fn next_nonce(&self, wallet: &Address) -> Result<u64, Error> {
        Ok(self.state.lock().unwrap().next_nonce(wallet))
    }

    async fn gas_price(&self) -> Result<u128, Error> {
        Ok(GAS_PRICE)
    }

    async fn send_transaction(
        &self,
        signed_transaction: &SignedTransaction,
        fees: Fees,
    ) -> Result<TxHash, Error> {
        let mut state = self.state.lock().unwrap();

        state.check_nonce(signed_transaction)?;

        // --- Balance: the sender pays the amount plus the fees ---------------

        let sender = signed_transaction.sender();
        let recipient = signed_transaction.recipient();
        let amount = signed_transaction.amount();

        let available = *state.balances.get(sender).unwrap_or(&0);
        let remaining = available
            .checked_sub(amount + fees.total())
            .ok_or_else(|| refused(format!("{sender} has only {available} wei")))?;

        state.balances.insert(sender.clone(), remaining);
        *state.balances.entry(recipient.clone()).or_insert(0) += amount;

        // --- Block -----------------------------------------------------------

        let gas_used = signed_transaction.gas_limit();

        Ok(state.add_block(signed_transaction, true, gas_used))
    }

    async fn send_failed_transaction(
        &self,
        signed_transaction: &SignedTransaction,
    ) -> Result<TxHash, Error> {
        let mut state = self.state.lock().unwrap();

        state.check_nonce(signed_transaction)?;

        Ok(state.add_block(signed_transaction, false, 0))
    }

    async fn send_cancellation(&self, cancellation: &SignedTransaction) -> Result<TxHash, Error> {
        let mut state = self.state.lock().unwrap();

        state.check_nonce(cancellation)?;

        Ok(state.add_block(cancellation, true, 0))
    }

    async fn block_number(&self) -> Result<u64, Error> {
        Ok(self.state.lock().unwrap().blocks.len() as u64 - 1)
    }

    async fn block(&self, number: u64) -> Result<Option<Block>, Error> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .blocks
            .get(number as usize)
            .cloned())
    }

    async fn block_by_hash(&self, hash: &str) -> Result<Option<Block>, Error> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .blocks
            .iter()
            .find(|block| block.hash == hash)
            .cloned())
    }

    async fn receipt(&self, tx_hash: &TxHash) -> Result<Option<Receipt>, Error> {
        Ok(self.state.lock().unwrap().receipts.get(tx_hash).cloned())
    }
}
