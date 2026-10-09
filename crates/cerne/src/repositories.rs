use crate::entities::Aggregate;
use crate::errors::Error;
use async_trait::async_trait;

/// The port that loads and saves an aggregate. Only an `Aggregate` fits: `Repository<OrderItem, Transaction>` does not
/// compile.
///
/// `save` works like in Rails: an aggregate without an id is inserted, and the repository decides its id; an aggregate
/// with an id is updated. Either way, `save` returns the id.
///
/// Both methods take the transaction the command opened (`composition_root.database.begin()`): `Transaction` is the
/// application's own type, the same for every repository and the event outbox of one composition root.
///
/// Not finding the aggregate is an `ApplicationError::NotFound`; a database failure is an `InfrastructureError`.
///
/// ```
/// use cerne::application::Repository;
/// use cerne::domain::{EnforcementResult, Entity, Validate, ValueObject, aggregate};
/// use cerne::{ApplicationError, Error, async_trait};
/// use std::collections::HashMap;
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
/// // In memory, the transaction holds the data itself.
/// #[derive(Default)]
/// struct InMemoryTransaction {
///     orders: HashMap<OrderId, Order>,
/// }
///
/// struct InMemoryOrderRepository;
///
/// #[async_trait]
/// impl Repository<Order, InMemoryTransaction> for InMemoryOrderRepository {
///     async fn load(&self, transaction: &mut InMemoryTransaction, id: &OrderId) -> Result<Order, Error> {
///         Ok(transaction.orders.get(id).cloned().ok_or(ApplicationError::NotFound("order"))?)
///     }
///
///     async fn save(&self, transaction: &mut InMemoryTransaction, order: Order) -> Result<OrderId, Error> {
///         let order_id = match order.id() {
///             Some(order_id) => order_id.clone(),
///             None => OrderId::new(transaction.orders.len() as u64 + 1)?,
///         };
///
///         transaction.orders.insert(order_id.clone(), order.with_id(order_id.clone()));
///
///         Ok(order_id)
///     }
/// }
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
/// let mut transaction = InMemoryTransaction::default();
///
/// let order_id = InMemoryOrderRepository.save(&mut transaction, Order::new(OrderConstructor {})?).await?;
///
/// assert!(order_id == OrderId(1));
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait Repository<A: Aggregate, Transaction: Send>: Send + Sync {
    async fn load(&self, transaction: &mut Transaction, id: &A::Id) -> Result<A, Error>;

    /// Inserts the aggregate if it has no id yet (deciding one), updates it otherwise; returns its id.
    async fn save(&self, transaction: &mut Transaction, aggregate: A) -> Result<A::Id, Error>;
}
