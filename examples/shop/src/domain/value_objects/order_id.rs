use cerne::domain::{DomainError, EnforcementResult, Invariant, Invariants, ValueObject};
use serde::{Deserialize, Serialize};

/// In JSON it is the `u64` itself, and reading it back goes through `new`: the invariants hold there too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct OrderId(u64);

impl ValueObject for OrderId {
    type Props = u64;

    fn new(value: u64) -> EnforcementResult<Self> {
        let order_id_is_positive = value > 0;

        Invariants::new(vec![Invariant::new("order id is positive", move || {
            order_id_is_positive
        })])
        .enforce()?;

        Ok(Self(value))
    }
}

impl TryFrom<u64> for OrderId {
    type Error = DomainError;

    fn try_from(value: u64) -> EnforcementResult<Self> {
        Self::new(value)
    }
}

impl From<OrderId> for u64 {
    fn from(value: OrderId) -> u64 {
        value.0
    }
}
