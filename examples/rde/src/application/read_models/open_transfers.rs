use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::tx_hash::TxHash;
use cerne::application::ReadModel;
use serde::Serialize;

/// The transfers of a sender whose transaction is in no block yet: pending, or answered and waiting for the chain.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OpenTransfers {
    pub sender: Address,
    pub transfers: Vec<TxHash>,
}

impl ReadModel for OpenTransfers {}
