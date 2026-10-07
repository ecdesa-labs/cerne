use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::tx_hash::TxHash;
use cerne::application::ReadModel;
use serde::Serialize;

/// What the recipient sees before accepting or rejecting: every transfer still waiting for them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PendingTransfers {
    pub recipient: Address,
    pub transfers: Vec<PendingTransfer>,
}

/// One line of the list: who sends, how much, and the tx_hash to accept or reject.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PendingTransfer {
    pub tx_hash: TxHash,
    pub sender: Address,
    /// In wei, as decimal text: a JSON number loses precision past 2^53, and 1 RDEC is already 10^18 wei.
    pub amount: String,
}

impl ReadModel for PendingTransfers {}
