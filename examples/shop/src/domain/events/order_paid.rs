use crate::domain::value_objects::order_id::OrderId;
use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies};

pub struct OrderPaid {
    pub order_id: OrderId,
}

impl DomainEvent<Ports> for OrderPaid {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        // --- Policies --------------------------------------------------------

        Ok(Policies::new(vec![]).trigger())
    }
}
