use crate::errors::EnforcementResult;
use crate::value_objects::ValueObject;

/// The yellow post-it: something with an identity that only exists while its invariants hold.
///
/// Like a record in Rails, an entity is born without an id: the repository decides it on the first `save`.
///
/// [`entity`](crate::domain::entity) (or [`aggregate`](crate::domain::aggregate)) writes this trait and the
/// `<Name>Constructor`; the invariants are yours, in [`Validate`].
///
/// ```
/// use cerne::domain::{EnforcementResult, Entity, Invariants, Validate, ValueObject, entity, invariant};
///
/// #[derive(Debug, Clone, PartialEq)]
/// struct OrderItemId(u64);
///
/// impl ValueObject for OrderItemId {
///     type Constructor = u64;
///
///     fn new(value: u64) -> EnforcementResult<Self> {
///         Ok(Self(value))
///     }
/// }
///
/// #[entity]
/// struct OrderItem {
///     id: Option<OrderItemId>,
///     qty: i32,
/// }
///
/// impl Validate for OrderItem {
///     fn validate(self) -> EnforcementResult<Self> {
///         let quantity_is_positive = self.qty > 0;
///
///         Invariants::enforce([invariant!("positive quantity", quantity_is_positive)])?;
///
///         Ok(self)
///     }
/// }
///
/// let order_item = OrderItem::new(OrderItemConstructor { qty: 3 }).unwrap();
///
/// assert!(order_item.id().is_none()); // not saved yet
/// assert!(OrderItem::new(OrderItemConstructor { qty: 0 }).is_err());
/// ```
///
/// Without the attribute, the trait is written by hand. That is the way for an entity whose id comes from its own
/// content, such as a hash or a document number, and so is never `None` (see the example of
/// [`Command`](crate::application::Command)).
pub trait Entity: Validate + Sized + Send + Sync {
    type Id: ValueObject;

    /// What a new entity is made of: with `#[entity]`, a `<Name>Constructor` with every field but the id.
    type Constructor;

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
    fn new(constructor: Self::Constructor) -> EnforcementResult<Self>;
}

/// The invariants of an entity: what is always true about it.
///
/// `new` ends here, and so does every state change (`Self { status, ..self }.validate()`), so the entity never
/// reaches an invalid state.
pub trait Validate: Sized {
    /// Gives the entity back only if its invariants hold.
    fn validate(self) -> EnforcementResult<Self>;
}

/// Marks the root of a consistency boundary: the only kind of entity a `Repository` can load and save.
///
/// [`aggregate`](crate::domain::aggregate) writes it, together with [`Entity`].
pub trait Aggregate: Entity {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::DomainError;
    use crate::invariant;
    use crate::invariants::Invariants;
    use cerne_macros::{aggregate, entity};

    #[derive(Debug, Clone, PartialEq)]
    struct OrderId(u64);

    impl ValueObject for OrderId {
        type Constructor = u64;

        fn new(value: u64) -> EnforcementResult<Self> {
            Ok(Self(value))
        }
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    enum OrderStatus {
        #[default]
        Placed,
    }

    #[aggregate]
    #[derive(Debug, Clone, PartialEq)]
    struct Order {
        id: Option<OrderId>,
        qty: i32,
        #[skip_constructor]
        status: OrderStatus,
    }

    impl Validate for Order {
        fn validate(self) -> EnforcementResult<Self> {
            let quantity_is_not_negative = self.qty >= 0;

            Invariants::enforce([invariant!("non negative quantity", quantity_is_not_negative)])?;

            Ok(self)
        }
    }

    #[entity]
    struct OrderLine {
        id: Option<OrderId>,
    }

    impl Validate for OrderLine {
        fn validate(self) -> EnforcementResult<Self> {
            Ok(self)
        }
    }

    fn is_aggregate<A: Aggregate>() {}

    #[test]
    fn entity_is_born_without_an_id_when_invariants_hold() {
        let order = Order::new(OrderConstructor { qty: 3 });

        assert_eq!(
            order,
            Ok(Order {
                id: None,
                qty: 3,
                status: OrderStatus::Placed
            })
        );
    }

    #[test]
    fn entity_cannot_be_born_invalid() {
        assert_eq!(
            Order::new(OrderConstructor { qty: -1 }),
            Err(DomainError::Violations(vec!["non negative quantity"]))
        );
    }

    #[test]
    fn entity_is_known_by_the_id_its_repository_gave() {
        let order = Order::new(OrderConstructor { qty: 3 })
            .unwrap()
            .with_id(OrderId(7));

        assert_eq!(order.id(), Some(&OrderId(7)));
    }

    #[test]
    fn an_entity_without_fields_besides_the_id_has_an_empty_constructor() {
        let order_line = OrderLine::new(OrderLineConstructor {}).unwrap();

        assert!(order_line.id().is_none());
    }

    #[test]
    fn aggregate_writes_impl_aggregate() {
        is_aggregate::<Order>();
    }
}
