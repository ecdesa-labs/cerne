use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy};

pub struct RecipientNotified {
    pub tx_hash: String,
    pub recipient: String,
}

impl DomainEvent<Ports> for RecipientNotified {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        Ok(vec![])
    }
}
