use crate::domain::value_objects::tx_hash::TxHash;
use cerne::application::ReadModel;
use serde::Serialize;

/// Whether the rde already has this transaction: as a transfer, or as the cancellation of one.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReceivedTransaction {
    pub tx_hash: TxHash,
    pub received: bool,
}

impl ReadModel for ReceivedTransaction {}
