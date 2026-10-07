use crate::domain_events::DomainEvent;
use crate::errors::Error;
use async_trait::async_trait;

/// The blue post-it: an intention that, once accepted by the domain, becomes domain events.
///
/// `Ports` is the composition root: the struct holding every port (repositories, external systems) the application uses.
/// `Output` is what the caller gets back (e.g. the id of what was created); `()` when there is nothing to give back.
///
/// ```
/// use cerne::application::{Command, Executed, Repository};
/// use cerne::domain::{
///     Aggregate, BusinessRule, BusinessRules, DomainEvent, EnforcementResult, Entity, FiredPolicy, ValueObject,
/// };
/// use cerne::{Error, async_trait};
///
/// #[derive(Clone, PartialEq)]
/// struct ProductId(u64);
///
/// impl ValueObject for ProductId {
///     type Props = u64;
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
/// impl Entity for Stock {
///     type Id = ProductId;
///     type Props = (ProductId, i32);
///
///     fn id(&self) -> Option<&ProductId> {
///         Some(&self.product_id) // the stock of a product is known by the product: it always has an id
///     }
///
///     fn with_id(self, product_id: ProductId) -> Self {
///         Self { product_id, ..self }
///     }
///
///     fn new((product_id, available): (ProductId, i32)) -> EnforcementResult<Self> {
///         Self { product_id, available }.validate()
///     }
///
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
/// impl DomainEvent<Ports> for StockReserved {
///     fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
///         Ok(vec![])
///     }
/// }
///
/// struct Ports {
///     stock: Box<dyn Repository<Stock>>,
/// }
///
/// struct ReserveStockCommand {
///     product_id: ProductId,
///     qty: i32,
/// }
///
/// #[async_trait]
/// impl Command<Ports> for ReserveStockCommand {
///     type Output = i32; // units left in stock
///
///     async fn execute(&self, ports: &Ports) -> Result<Executed<i32, Ports>, Error> {
///         // --- Ports -----------------------------------------------------------
///
///         let stock = ports.stock.load(&self.product_id).await?;
///
///         // --- Business rules --------------------------------------------------
///
///         let enough_stock = stock.available >= self.qty;
///
///         BusinessRules::new(vec![BusinessRule::new("enough stock", move || enough_stock)]).check()?;
///
///         // --- Aggregate -------------------------------------------------------
///
///         let stock = Stock::new((stock.product_id, stock.available - self.qty))?;
///         let units_left = stock.available;
///
///         ports.stock.save(stock).await?;
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
pub trait Command<Ports>: Send + Sync {
    type Output: Send;

    /// Either rejects the command (and nothing changes) or applies it and returns its output and the events it produced.
    async fn execute(&self, ports: &Ports) -> Result<Executed<Self::Output, Ports>, Error>;
}

/// What an accepted command gives back: its own result for the caller, and the events it produced for the policies.
pub struct Executed<Output, Ports> {
    pub output: Output,
    pub events: Vec<Box<dyn DomainEvent<Ports>>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business_rules::{BusinessRule, BusinessRules};
    use crate::entities::Entity;
    use crate::errors::{DomainError, EnforcementResult};
    use crate::invariants::{Invariant, Invariants};
    use crate::policies::FiredPolicy;
    use crate::value_objects::ValueObject;
    use std::sync::Mutex;

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
            let at_most_1000_items = self.qty <= 1000;

            Invariants::new(vec![Invariant::new("at most 1000 items", move || {
                at_most_1000_items
            })])
            .enforce()?;

            Ok(self)
        }
    }

    struct Ports {
        order: Mutex<Order>,
    }

    struct OrderPlaced;

    impl DomainEvent<Ports> for OrderPlaced {
        fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
            Ok(vec![])
        }
    }

    struct PlaceOrderCommand {
        qty: i32,
    }

    #[async_trait]
    impl Command<Ports> for PlaceOrderCommand {
        type Output = ();

        async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
            let quantity_is_positive = self.qty > 0;

            BusinessRules::new(vec![BusinessRule::new("positive quantity", move || {
                quantity_is_positive
            })])
            .check()?;

            let mut order = ports.order.lock().unwrap();
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

    fn ports() -> Ports {
        Ports {
            order: Mutex::new(Order {
                id: Some(OrderId(1)),
                qty: 0,
            }),
        }
    }

    fn violations(result: Result<Executed<(), Ports>, Error>) -> Vec<&'static str> {
        match result {
            Err(Error::Domain(DomainError::Violations(v))) => v,
            _ => panic!("expected a domain error"),
        }
    }

    #[tokio::test]
    async fn command_changes_the_entity_and_produces_a_domain_event() {
        let ports = ports();

        let place_order_execution = PlaceOrderCommand { qty: 3 }.execute(&ports).await.unwrap();

        assert_eq!(place_order_execution.events.len(), 1);
        assert_eq!(
            *ports.order.lock().unwrap(),
            Order {
                id: Some(OrderId(1)),
                qty: 3
            }
        );
    }

    #[tokio::test]
    async fn command_is_rejected_by_a_business_rule() {
        let ports = ports();

        let result = PlaceOrderCommand { qty: -1 }.execute(&ports).await;

        assert_eq!(violations(result), vec!["positive quantity"]);
        assert_eq!(
            *ports.order.lock().unwrap(),
            Order {
                id: Some(OrderId(1)),
                qty: 0
            }
        );
    }

    #[tokio::test]
    async fn command_is_rejected_by_an_invariant() {
        let ports = ports();

        let result = PlaceOrderCommand { qty: 2000 }.execute(&ports).await;

        assert_eq!(violations(result), vec!["at most 1000 items"]);
        assert_eq!(
            *ports.order.lock().unwrap(),
            Order {
                id: Some(OrderId(1)),
                qty: 0
            }
        );
    }
}
