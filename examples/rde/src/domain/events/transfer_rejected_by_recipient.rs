use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy};

pub struct TransferRejectedByRecipient {
    pub tx_hash: TxHash,
}

impl DomainEvent<Ports> for TransferRejectedByRecipient {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        Ok(vec![])
    }
}
