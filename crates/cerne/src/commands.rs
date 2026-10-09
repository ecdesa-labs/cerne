use crate::domain_events::DomainEvent;
use crate::errors::Error;
use async_trait::async_trait;

/// The blue post-it: an intention that, once accepted by the domain, becomes domain events.
///
/// `CompositionRoot` is the composition root: the struct holding every port (repositories, external systems) the application uses.
/// `Output` is what the caller gets back (e.g. the id of what was created); `()` when there is nothing to give back.
///
/// ```
/// use cerne::application::{Command, Executed, Repository};
/// use cerne::domain::{
///     Aggregate, BusinessRules, DomainEvent, EnforcementResult, Entity, FiredPolicy, Validate, ValueObject,
///     business_rule,
/// };
/// use cerne::{Error, async_trait};
///
/// #[derive(Clone, PartialEq)]
/// struct ProductId(u64);
///
/// impl ValueObject for ProductId {
///     type Constructor = u64;
///
///     fn new(value: u64) -> EnforcementResult<Self> {
///         Ok(Self(value))
///     }
/// }
///
/// #[derive(Clone)]
/// struct Stock {
///     product_id: ProductId,
///     available: i32,
/// }
///
/// // The stock of a product is known by the product: its id is never `None`, so `Entity` is written by hand.
/// impl Entity for Stock {
///     type Id = ProductId;
///     type Constructor = (ProductId, i32);
///
///     fn id(&self) -> Option<&ProductId> {
///         Some(&self.product_id)
///     }
///
///     fn with_id(self, product_id: ProductId) -> Self {
///         Self { product_id, ..self }
///     }
///
///     fn new((product_id, available): (ProductId, i32)) -> EnforcementResult<Self> {
///         Self { product_id, available }.validate()
///     }
/// }
///
/// impl Validate for Stock {
///     fn validate(self) -> EnforcementResult<Self> {
///         Ok(self)
///     }
/// }
///
/// impl Aggregate for Stock {}
///
/// struct StockReserved {
///     product_id: ProductId,
///     qty: i32,
/// }
///
/// impl DomainEvent<CompositionRoot> for StockReserved {
///     fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> {
///         Ok(vec![])
///     }
/// }
///
/// struct CompositionRoot {
///     stock_repository: Box<dyn Repository<Stock>>,
/// }
///
/// struct ReserveStockCommand {
///     product_id: ProductId,
///     qty: i32,
/// }
///
/// #[async_trait]
/// impl Command<CompositionRoot> for ReserveStockCommand {
///     type Output = i32; // units left in stock
///
///     async fn execute(&self, composition_root: &CompositionRoot) -> Result<Executed<i32, CompositionRoot>, Error> {
///         // --- Ports -----------------------------------------------------------
///
///         let stock = composition_root.stock_repository.load(&self.product_id).await?;
///
///         // --- Business rules --------------------------------------------------
///
///         let enough_stock = stock.available >= self.qty;
///
///         BusinessRules::check([business_rule!("enough stock", enough_stock)])?;
///
///         // --- Aggregate -------------------------------------------------------
///
///         let stock = Stock::new((stock.product_id, stock.available - self.qty))?;
///         let units_left = stock.available;
///
///         composition_root.stock_repository.save(stock).await?;
///
///         // --- Domain events ---------------------------------------------------
///
///         let stock_reserved = StockReserved { product_id: self.product_id.clone(), qty: self.qty };
///
///         Ok(Executed {
///             output: units_left,
///             events: vec![Box::new(stock_reserved)],
///         })
///     }
/// }
/// ```
#[async_trait]
pub trait Command<CompositionRoot>: Send + Sync {
    type Output: Send;

    /// Either rejects the command (and nothing changes) or applies it and returns its output and the events it produced.
    async fn execute(
        &self,
        composition_root: &CompositionRoot,
    ) -> Result<Executed<Self::Output, CompositionRoot>, Error>;
}

/// What an accepted command gives back: its own result for the caller, and the events it produced for the policies.
pub struct Executed<Output, CompositionRoot> {
    pub output: Output,
    pub events: Vec<Box<dyn DomainEvent<CompositionRoot>>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business_rule;
    use crate::business_rules::BusinessRules;
    use crate::entities::Validate;
    use crate::errors::{DomainError, EnforcementResult};
    use crate::invariant;
    use crate::invariants::Invariants;
    use crate::policies::FiredPolicy;
    use crate::value_objects::ValueObject;
    use cerne_macros::entity;
    use std::sync::Mutex;

    #[derive(Debug, Clone, PartialEq)]
    struct OrderId(u64);

    impl ValueObject for OrderId {
        type Constructor = u64;

        fn new(value: u64) -> EnforcementResult<Self> {
            Ok(Self(value))
        }
    }

    #[entity]
    #[derive(Debug, Clone, PartialEq)]
    struct Order {
        id: Option<OrderId>,
        qty: i32,
    }

    impl Validate for Order {
        fn validate(self) -> EnforcementResult<Self> {
            let at_most_1000_items = self.qty <= 1000;

            Invariants::enforce([invariant!("at most 1000 items", at_most_1000_items)])?;

            Ok(self)
        }
    }

    struct CompositionRoot {
        order: Mutex<Order>,
    }

    struct OrderPlaced;

    impl DomainEvent<CompositionRoot> for OrderPlaced {
        fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> {
            Ok(vec![])
        }
    }

    struct PlaceOrderCommand {
        qty: i32,
    }

    #[async_trait]
    impl Command<CompositionRoot> for PlaceOrderCommand {
        type Output = ();

        async fn execute(&self, composition_root: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
            let quantity_is_positive = self.qty > 0;

            BusinessRules::check([business_rule!("positive quantity", quantity_is_positive)])?;

            let mut order = composition_root.order.lock().unwrap();
            *order = Order {
                qty: self.qty,
                ..order.clone()
            }
            .validate()?;

            let order_placed = OrderPlaced;

            Ok(Executed {
                output: (),
                events: vec![Box::new(order_placed)],
            })
        }
    }

    fn composition_root() -> CompositionRoot {
        CompositionRoot {
            order: Mutex::new(Order {
                id: Some(OrderId(1)),
                qty: 0,
            }),
        }
    }

    fn violations(result: Result<Executed<(), CompositionRoot>, Error>) -> Vec<&'static str> {
        match result {
            Err(Error::Domain(DomainError::Violations(v))) => v,
            _ => panic!("expected a domain error"),
        }
    }

    #[tokio::test]
    async fn command_changes_the_entity_and_produces_a_domain_event() {
        let composition_root = composition_root();

        let place_order_execution = PlaceOrderCommand { qty: 3 }
            .execute(&composition_root)
            .await
            .unwrap();

        assert_eq!(place_order_execution.events.len(), 1);
        assert_eq!(
            *composition_root.order.lock().unwrap(),
            Order {
                id: Some(OrderId(1)),
                qty: 3
            }
        );
    }

    #[tokio::test]
    async fn command_is_rejected_by_a_business_rule() {
        let composition_root = composition_root();

        let result = PlaceOrderCommand { qty: -1 }
            .execute(&composition_root)
            .await;

        assert_eq!(violations(result), vec!["positive quantity"]);
        assert_eq!(
            *composition_root.order.lock().unwrap(),
            Order {
                id: Some(OrderId(1)),
                qty: 0
            }
        );
    }

    #[tokio::test]
    async fn command_is_rejected_by_an_invariant() {
        let composition_root = composition_root();

        let result = PlaceOrderCommand { qty: 2000 }
            .execute(&composition_root)
            .await;

        assert_eq!(violations(result), vec!["at most 1000 items"]);
        assert_eq!(
            *composition_root.order.lock().unwrap(),
            Order {
                id: Some(OrderId(1)),
                qty: 0
            }
        );
    }
}
