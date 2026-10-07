use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy};

pub struct RecipientNotified {
    pub tx_hash: TxHash,
    pub recipient: String,
}

impl DomainEvent<Ports> for RecipientNotified {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        Ok(vec![])
    }
}
