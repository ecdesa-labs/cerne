use crate::errors::{DomainError, EnforcementResult};

/// What is always true about an entity or a value object: a name, as written on the board, and whether it holds. Written with [`invariant!`](crate::domain::invariant).
pub struct Invariant {
    name: &'static str,
    holds: Box<dyn Fn() -> bool + Send + Sync>,
}

impl Invariant {
    pub fn new(name: &'static str, holds: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            name,
            holds: Box::new(holds),
        }
    }
}

/// Runs the invariants of an entity or a value object: `Invariants::enforce([invariant!(..), ..])?`.
pub struct Invariants;

impl Invariants {
    /// Runs every invariant and, if any fails, returns `DomainError::Violations` with every failing name, not just
    /// the first.
    pub fn enforce(invariants: impl IntoIterator<Item = Invariant>) -> EnforcementResult<()> {
        let violations: Vec<_> = invariants
            .into_iter()
            .filter(|invariant| !(invariant.holds)())
            .map(|invariant| invariant.name)
            .collect();

        if violations.is_empty() { Ok(()) } else { Err(DomainError::Violations(violations)) }
    }
}

/// An [`Invariant`](crate::domain::Invariant): `invariant!("quantity is positive", quantity_is_positive)`.
///
/// The condition goes in a variable before, named like the sentence on the board; the macro reads it once, right
/// away, and writes `Invariant::new(name, move || condition)`.
///
/// ```
/// use cerne::domain::{Invariants, invariant};
///
/// let quantity = 0;
/// let quantity_is_positive = quantity > 0;
///
/// assert!(Invariants::enforce([invariant!("quantity is positive", quantity_is_positive)]).is_err());
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! invariant {
    ($name:expr, $holds:expr $(,)?) => {{
        let holds: bool = $holds;

        $crate::domain::Invariant::new($name, move || holds)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforce_is_ok_when_all_hold() {
        let quantity_is_positive = true;

        assert_eq!(Invariants::enforce([invariant!("quantity is positive", quantity_is_positive)]), Ok(()));
    }

    #[test]
    fn enforce_returns_every_violation() {
        let order_has_a_product = false;
        let quantity_is_positive = false;

        assert_eq!(
            Invariants::enforce([
                invariant!("order has a product", order_has_a_product),
                invariant!("quantity is positive", quantity_is_positive),
            ]),
            Err(DomainError::Violations(vec!["order has a product", "quantity is positive"]))
        );
    }

    #[test]
    fn enforce_is_ok_without_invariants() {
        assert_eq!(Invariants::enforce([]), Ok(()));
    }
}
