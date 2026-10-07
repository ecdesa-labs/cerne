use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy};

/// The cancellation signed in MetaMask is in a block, as succeeded: the nonce of the sender moved on.
pub struct CancellationChained {
    pub tx_hash: TxHash,
}

impl DomainEvent<Ports> for CancellationChained {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        Ok(vec![])
    }
}
