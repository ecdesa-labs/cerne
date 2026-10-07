use crate::errors::EnforcementResult;
use crate::value_objects::ValueObject;

/// The yellow post-it: something with an identity that only exists while its invariants hold.
///
/// Like a record in Rails, an entity is born without an id: the repository decides it on the first `save`.
///
/// ```
/// use cerne::domain::{EnforcementResult, Entity, Invariant, Invariants, ValueObject};
///
/// #[derive(Debug, Clone, PartialEq)]
/// struct OrderItemId(u64);
///
/// impl ValueObject for OrderItemId {
///     type Props = u64;
///
///     fn new(value: u64) -> EnforcementResult<Self> {
///         Ok(Self(value))
///     }
/// }
///
/// struct OrderItem {
///     id: Option<OrderItemId>,
///     qty: i32,
/// }
///
/// impl Entity for OrderItem {
///     type Id = OrderItemId;
///     type Props = i32;
///
///     fn id(&self) -> Option<&OrderItemId> {
///         self.id.as_ref()
///     }
///
///     fn with_id(self, id: OrderItemId) -> Self {
///         Self { id: Some(id), ..self }
///     }
///
///     fn new(qty: i32) -> EnforcementResult<Self> {
///         Self { id: None, qty }.validate()
///     }
///
///     fn validate(self) -> EnforcementResult<Self> {
///         let quantity_is_positive = self.qty > 0;
///
///         Invariants::new(vec![Invariant::new("positive quantity", move || {
///             quantity_is_positive
///         })])
///         .enforce()?;
///
///         Ok(self)
///     }
/// }
///
/// let order_item = OrderItem::new(3).unwrap();
///
/// assert!(order_item.id().is_none()); // not saved yet
/// assert!(OrderItem::new(0).is_err());
/// ```
pub trait Entity: Sized + Send + Sync {
    type Id: ValueObject;
    type Props;

    /// The identity: two entities with the same id are the same entity, whatever their other fields.
    ///
    /// `None` until the repository saves the entity for the first time. An entity whose id comes from its own content
    /// (a hash, a document number) always returns `Some`.
    fn id(&self) -> Option<&Self::Id>;

    /// Gives the entity the id its repository decided. Only a `Repository` calls it, on the first `save`.
    fn with_id(self, id: Self::Id) -> Self;

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
/// # use cerne::domain::{EnforcementResult, Entity, ValueObject};
/// # #[derive(Clone, PartialEq)]
/// # struct OrderId(u64);
/// # impl ValueObject for OrderId {
/// #     type Props = u64;
/// #     fn new(value: u64) -> EnforcementResult<Self> { Ok(Self(value)) }
/// # }
/// # struct Order { id: Option<OrderId> }
/// # impl Entity for Order {
/// #     type Id = OrderId;
/// #     type Props = ();
/// #     fn id(&self) -> Option<&OrderId> { self.id.as_ref() }
/// #     fn with_id(self, id: OrderId) -> Self { Self { id: Some(id) } }
/// #     fn new(_: ()) -> EnforcementResult<Self> { Ok(Self { id: None }) }
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
    struct OrderId(u64);

    impl ValueObject for OrderId {
        type Props = u64;

        fn new(value: u64) -> EnforcementResult<Self> {
            Ok(Self(value))
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct Order {
        id: Option<OrderId>,
        qty: i32,
    }

    impl Entity for Order {
        type Id = OrderId;
        type Props = i32;

        fn id(&self) -> Option<&OrderId> {
            self.id.as_ref()
        }

        fn with_id(self, id: OrderId) -> Self {
            Self {
                id: Some(id),
                ..self
            }
        }

        fn new(qty: i32) -> EnforcementResult<Self> {
            Self { id: None, qty }.validate()
        }

        fn validate(self) -> EnforcementResult<Self> {
            let quantity_is_not_negative = self.qty >= 0;

            Invariants::new(vec![Invariant::new("non negative quantity", move || {
                quantity_is_not_negative
            })])
            .enforce()?;

            Ok(self)
        }
    }

    #[test]
    fn entity_is_born_without_an_id_when_invariants_hold() {
        assert_eq!(Order::new(3), Ok(Order { id: None, qty: 3 }));
    }

    #[test]
    fn entity_cannot_be_born_invalid() {
        assert_eq!(
            Order::new(-1),
            Err(DomainError::Violations(vec!["non negative quantity"]))
        );
    }

    #[test]
    fn entity_is_known_by_the_id_its_repository_gave() {
        let order = Order::new(3).unwrap().with_id(OrderId(7));

        assert_eq!(order.id(), Some(&OrderId(7)));
    }
}
