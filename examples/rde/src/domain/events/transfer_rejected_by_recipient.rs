use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy};

pub struct TransferRejectedByRecipient {
    pub tx_hash: String,
}

impl DomainEvent<Ports> for TransferRejectedByRecipient {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        Ok(vec![])
    }
}
