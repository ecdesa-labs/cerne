use crate::application::commands::charge_order::ChargeOrderCommand;
use crate::domain::value_objects::order_id::OrderId;
use crate::ports::Ports;
use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, Policy};

pub struct OrderPlaced {
    pub order_id: OrderId,
    pub total: u64,
}

impl DomainEvent<Ports> for OrderPlaced {
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
        // --- Policies --------------------------------------------------------

        let order_id = self.order_id.clone();
        let total = self.total;

        let charge_the_customer_policy = Policy::new(
            "whenever an order is placed, charge the customer",
            || true,
            move || {
                Box::new(ChargeOrderCommand {
                    order_id: order_id.clone(),
                    total,
                })
            },
        );

        Ok(Policies::new(vec![charge_the_customer_policy]).trigger())
    }
}
