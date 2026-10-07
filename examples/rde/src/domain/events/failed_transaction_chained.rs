use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy};

/// The transaction of a rejected or canceled transfer is in a block, as failed: the nonce of the sender moved on.
pub struct FailedTransactionChained {
    pub tx_hash: TxHash,
}

impl DomainEvent<Ports> for FailedTransactionChained {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        Ok(vec![])
    }
}
