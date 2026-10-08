use crate::domain::value_objects::order_id::OrderId;
use cerne::domain::{Aggregate, EnforcementResult, Entity, Invariant, Invariants};

// --- Status ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderStatus {
    Placed,
    Paid,
}

// --- Aggregate ---------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub id: Option<OrderId>,
    pub product: String,
    pub quantity: u32,
    pub total: u64,
    pub status: OrderStatus,
}

/// What a new `Order` is made of. There is no id: the repository decides it on the first `save`.
pub struct OrderProps {
    pub product: String,
    pub quantity: u32,
    pub total: u64,
}

impl Aggregate for Order {}

// --- Entity: identity and invariants -----------------------------------------

impl Entity for Order {
    type Id = OrderId;
    type Props = OrderProps;

    fn id(&self) -> Option<&OrderId> {
        self.id.as_ref()
    }

    fn with_id(self, id: OrderId) -> Self {
        Self {
            id: Some(id),
            ..self
        }
    }

    fn new(props: OrderProps) -> EnforcementResult<Self> {
        Self {
            id: None,
            product: props.product,
            quantity: props.quantity,
            total: props.total,
            status: OrderStatus::Placed,
        }
        .validate()
    }

    fn validate(self) -> EnforcementResult<Self> {
        let order_has_a_product = !self.product.is_empty();
        let quantity_is_positive = self.quantity > 0;

        Invariants::new(vec![
            Invariant::new("order has a product", move || order_has_a_product),
            Invariant::new("quantity is positive", move || quantity_is_positive),
        ])
        .enforce()?;

        Ok(self)
    }
}

// --- State transitions -------------------------------------------------------

impl Order {
    pub fn pay(self) -> EnforcementResult<Self> {
        Self {
            status: OrderStatus::Paid,
            ..self
        }
        .validate()
    }
}
