use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy};

pub struct TransferCanceledBySender {
    pub tx_hash: String,
}

impl DomainEvent<Ports> for TransferCanceledBySender {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        Ok(vec![])
    }
}
