use crate::domain::entities::order::{Order, OrderConstructor};
use crate::domain::events::order_placed::OrderPlaced;
use crate::domain::value_objects::order_id::OrderId;
use crate::ports::Ports;
use cerne::application::{Command, Executed};
use cerne::domain::{BusinessRules, Entity, business_rule};
use cerne::{Error, async_trait};
use serde::{Deserialize, Serialize};

/// Actor: the customer. The body of `POST /orders` is this command (that is why it is `Deserialize`).
#[derive(Serialize, Deserialize)]
pub struct PlaceOrderCommand {
    pub product: String,
    pub quantity: u32,
}

#[async_trait]
impl Command<Ports> for PlaceOrderCommand {
    type Output = OrderId; // the id of the new order, for the customer to follow it

    async fn execute(&self, ports: &Ports) -> Result<Executed<OrderId, Ports>, Error> {
        // --- Ports -----------------------------------------------------------

        let unit_price = ports.catalog.unit_price(&self.product).await?;
        let units_in_stock = ports.catalog.units_in_stock(&self.product).await?;

        // --- Business rules --------------------------------------------------

        let stock_covers_the_quantity = units_in_stock >= self.quantity;

        BusinessRules::check([business_rule!("stock covers the quantity", stock_covers_the_quantity)])?;

        // --- Aggregate -------------------------------------------------------

        let total = unit_price * u64::from(self.quantity);

        let order = Order::new(OrderConstructor {
            product: self.product.clone(),
            quantity: self.quantity,
            total,
        })?;

        let order_id = ports.orders.save(order).await?;

        // --- Domain events ---------------------------------------------------

        let order_placed = OrderPlaced {
            order_id: order_id.clone(),
            total,
        };

        Ok(Executed {
            output: order_id,
            events: vec![Box::new(order_placed)],
        })
    }
}
