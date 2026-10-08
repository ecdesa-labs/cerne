use crate::domain::entities::order::OrderStatus;
use crate::domain::events::order_paid::OrderPaid;
use crate::domain::value_objects::order_id::OrderId;
use crate::ports::Ports;
use cerne::application::{Command, Executed};
use cerne::domain::{BusinessRule, BusinessRules};
use cerne::{Error, async_trait};
use serde::{Deserialize, Serialize};

/// No actor: a policy fires this command, and the outbox stores it (that is why it is `Serialize`).
#[derive(Serialize, Deserialize)]
pub struct ChargeOrderCommand {
    pub order_id: OrderId,
    pub total: u64,
}

#[async_trait]
impl Command<Ports> for ChargeOrderCommand {
    type Output = ();

    async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
        // --- Ports -----------------------------------------------------------

        let order = ports.orders.load(&self.order_id).await?;

        // --- Business rules --------------------------------------------------

        let order_is_still_placed = order.status == OrderStatus::Placed;

        BusinessRules::new(vec![BusinessRule::new(
            "order is still placed",
            move || order_is_still_placed,
        )])
        .check()?;

        // --- External system: Payments ---------------------------------------

        ports.payments.charge(&self.order_id, self.total).await?;

        // --- Aggregate -------------------------------------------------------

        let paid_order = order.pay()?;

        ports.orders.save(paid_order).await?;

        // --- Domain events ---------------------------------------------------

        let order_paid = OrderPaid {
            order_id: self.order_id.clone(),
        };

        Ok(Executed {
            output: (),
            events: vec![Box::new(order_paid)],
        })
    }
}
