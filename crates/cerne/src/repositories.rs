use crate::entities::Aggregate;
use crate::errors::Error;
use async_trait::async_trait;

/// The port that loads and saves an aggregate. Only an `Aggregate` fits: `Repository<OrderItem>` does not compile.
///
/// Not finding the aggregate is an `ApplicationError::NotFound`; a database failure is an `InfrastructureError`.
///
/// ```
/// use cerne::application::Repository;
/// use cerne::domain::{Aggregate, EnforcementResult, Entity};
/// use cerne::{ApplicationError, Error, async_trait};
/// use std::collections::HashMap;
/// use std::sync::Mutex;
///
/// #[derive(Clone)]
/// struct Order {
///     id: u64,
/// }
///
/// impl Entity for Order {
///     type Id = u64;
///     type Props = u64;
///
///     fn id(&self) -> &u64 {
///         &self.id
///     }
///
///     fn new(id: u64) -> EnforcementResult<Self> {
///         Ok(Self { id })
///     }
///
///     fn validate(self) -> EnforcementResult<Self> {
///         Ok(self)
///     }
/// }
///
/// impl Aggregate for Order {}
///
/// #[derive(Default)]
/// struct InMemoryOrders(Mutex<HashMap<u64, Order>>);
///
/// #[async_trait]
/// impl Repository<Order> for InMemoryOrders {
///     async fn load(&self, id: &u64) -> Result<Order, Error> {
///         let orders = self.0.lock().unwrap();
///
///         Ok(orders.get(id).cloned().ok_or(ApplicationError::NotFound("order"))?)
///     }
///
///     async fn save(&self, order: Order) -> Result<(), Error> {
///         self.0.lock().unwrap().insert(order.id, order);
///
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Repository<A: Aggregate>: Send + Sync {
    async fn load(&self, id: &A::Id) -> Result<A, Error>;

    async fn save(&self, aggregate: A) -> Result<(), Error>;
}
