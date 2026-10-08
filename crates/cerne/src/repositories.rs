use crate::entities::Aggregate;
use crate::errors::Error;
use async_trait::async_trait;

/// The port that loads and saves an aggregate. Only an `Aggregate` fits: `Repository<OrderItem>` does not compile.
///
/// `save` works like in Rails: an aggregate without an id is inserted, and the repository decides its id; an aggregate
/// with an id is updated. Either way, `save` returns the id.
///
/// Not finding the aggregate is an `ApplicationError::NotFound`; a database failure is an `InfrastructureError`.
///
/// ```
/// use cerne::application::Repository;
/// use cerne::domain::{EnforcementResult, Entity, Validate, ValueObject, aggregate};
/// use cerne::{ApplicationError, Error, async_trait};
/// use std::collections::HashMap;
/// use std::sync::Mutex;
///
/// #[derive(Clone, PartialEq, Eq, Hash)]
/// struct OrderId(u64);
///
/// impl ValueObject for OrderId {
///     type Constructor = u64;
///
///     fn new(value: u64) -> EnforcementResult<Self> {
///         Ok(Self(value))
///     }
/// }
///
/// #[aggregate]
/// #[derive(Clone)]
/// struct Order {
///     id: Option<OrderId>,
/// }
///
/// impl Validate for Order {
///     fn validate(self) -> EnforcementResult<Self> {
///         Ok(self)
///     }
/// }
///
/// #[derive(Default)]
/// struct InMemoryOrders(Mutex<HashMap<OrderId, Order>>);
///
/// #[async_trait]
/// impl Repository<Order> for InMemoryOrders {
///     async fn load(&self, id: &OrderId) -> Result<Order, Error> {
///         let orders = self.0.lock().unwrap();
///
///         Ok(orders.get(id).cloned().ok_or(ApplicationError::NotFound("order"))?)
///     }
///
///     async fn save(&self, order: Order) -> Result<OrderId, Error> {
///         let mut orders = self.0.lock().unwrap();
///
///         let order_id = match order.id() {
///             Some(order_id) => order_id.clone(),
///             None => OrderId::new(orders.len() as u64 + 1)?,
///         };
///
///         orders.insert(order_id.clone(), order.with_id(order_id.clone()));
///
///         Ok(order_id)
///     }
/// }
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
/// let orders = InMemoryOrders::default();
///
/// let order_id = orders.save(Order::new(OrderConstructor {})?).await?;
///
/// assert!(order_id == OrderId(1));
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait Repository<A: Aggregate>: Send + Sync {
    async fn load(&self, id: &A::Id) -> Result<A, Error>;

    /// Inserts the aggregate if it has no id yet (deciding one), updates it otherwise; returns its id.
    async fn save(&self, aggregate: A) -> Result<A::Id, Error>;
}
