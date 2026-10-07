use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy};

pub struct TransactionChained {
    pub tx_hash: TxHash,
}

impl DomainEvent<Ports> for TransactionChained {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        Ok(vec![])
    }
}
