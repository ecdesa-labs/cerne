use crate::errors::EnforcementResult;

/// The yellow post-it: something with an identity that only exists while its invariants hold.
///
/// ```
/// use cerne::domain::{EnforcementResult, Entity, Invariant, Invariants};
///
/// struct OrderItem {
///     id: u64,
///     qty: i32,
/// }
///
/// impl Entity for OrderItem {
///     type Id = u64;
///     type Props = (u64, i32);
///
///     fn id(&self) -> &u64 {
///         &self.id
///     }
///
///     fn new((id, qty): (u64, i32)) -> EnforcementResult<Self> {
///         Self { id, qty }.validate()
///     }
///
///     fn validate(self) -> EnforcementResult<Self> {
///         let qty = self.qty;
///
///         Invariants::new(vec![Invariant::new("positive quantity", move || qty > 0)]).enforce()?;
///
///         Ok(self)
///     }
/// }
///
/// assert!(OrderItem::new((1, 3)).is_ok());
/// assert!(OrderItem::new((1, 0)).is_err());
/// ```
pub trait Entity: Sized + Send + Sync {
    type Id: PartialEq + Send + Sync;
    type Props;

    /// The identity: two entities with the same id are the same entity, whatever their other fields.
    fn id(&self) -> &Self::Id;

    /// An entity is born valid: it only exists if its invariants hold.
    ///
    /// Otherwise nothing is created, and the error names every violated invariant, not just the first.
    fn new(props: Self::Props) -> EnforcementResult<Self>;

    /// Gives the entity back only if its invariants hold.
    ///
    /// Every state change ends here (`Self { status, ..self }.validate()`), so the entity never reaches an invalid state.
    fn validate(self) -> EnforcementResult<Self>;
}

/// Marks the root of a consistency boundary: the only kind of entity a `Repository` can load and save.
///
/// ```
/// # use cerne::domain::{EnforcementResult, Entity};
/// # struct Order { id: u64 }
/// # impl Entity for Order {
/// #     type Id = u64;
/// #     type Props = u64;
/// #     fn id(&self) -> &u64 { &self.id }
/// #     fn new(id: u64) -> EnforcementResult<Self> { Ok(Self { id }) }
/// #     fn validate(self) -> EnforcementResult<Self> { Ok(self) }
/// # }
/// use cerne::domain::Aggregate;
///
/// impl Aggregate for Order {}
/// ```
pub trait Aggregate: Entity {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::DomainError;
    use crate::invariants::{Invariant, Invariants};

    #[derive(Debug, Clone, PartialEq)]
    struct Order {
        id: u64,
        qty: i32,
    }

    impl Entity for Order {
        type Id = u64;
        type Props = (u64, i32);

        fn id(&self) -> &u64 {
            &self.id
        }

        fn new((id, qty): Self::Props) -> EnforcementResult<Self> {
            Self { id, qty }.validate()
        }

        fn validate(self) -> EnforcementResult<Self> {
            let qty = self.qty;

            Invariants::new(vec![Invariant::new("non negative quantity", move || {
                qty >= 0
            })])
            .enforce()?;

            Ok(self)
        }
    }

    #[test]
    fn entity_is_born_when_invariants_hold() {
        assert_eq!(Order::new((1, 3)), Ok(Order { id: 1, qty: 3 }));
    }

    #[test]
    fn entity_cannot_be_born_invalid() {
        assert_eq!(
            Order::new((1, -1)),
            Err(DomainError::Violations(vec!["non negative quantity"]))
        );
    }

    #[test]
    fn entity_is_known_by_its_id() {
        let order = Order::new((7, 3)).unwrap();

        assert_eq!(order.id(), &7);
    }
}
