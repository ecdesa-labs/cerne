use crate::application::ports::payments::Payments;
use crate::domain::value_objects::order_id::OrderId;
use cerne::{Error, async_trait};

/// The client of the payment provider's HTTP API: writing it is yours. Send the order id as the idempotency key.
pub struct HttpPayments;

#[async_trait]
impl Payments for HttpPayments {
    async fn charge(&self, _order_id: &OrderId, _amount: u64) -> Result<(), Error> {
        todo!("charge the customer on the payment provider")
    }
}
