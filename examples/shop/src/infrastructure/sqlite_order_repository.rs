use crate::domain::entities::order::{Order, OrderStatus};
use crate::domain::value_objects::order_id::OrderId;
use cerne::application::Repository;
use cerne::domain::{Entity, Validate, ValueObject};
use cerne::sqlite::{SqliteDatabase, column};
use cerne::{ApplicationError, Error, InfrastructureError, async_trait};

/// The `Order` aggregates in the table `orders` (`migrations/1791498533_create_orders.sql`).
pub struct SqliteOrderRepository {
    database: SqliteDatabase,
}

impl SqliteOrderRepository {
    pub fn new(database: SqliteDatabase) -> Self {
        Self { database }
    }
}

#[async_trait]
impl Repository<Order> for SqliteOrderRepository {
    async fn load(&self, id: &OrderId) -> Result<Order, Error> {
        let select = sqlx::query("SELECT * FROM orders WHERE id = $1").bind(u64::from(id.clone()) as i64);

        let row = self
            .database
            .fetch_optional(select)
            .await?
            .ok_or(ApplicationError::NotFound("order"))?;

        // Back through `validate`: a row that breaks an invariant is an error, not an aggregate.
        let order = Order {
            id: Some(OrderId::new(column::<i64>(&row, "id")? as u64)?),
            product: column::<String>(&row, "product")?,
            quantity: column::<i64>(&row, "quantity")? as u32,
            total: column::<i64>(&row, "total")? as u64,
            status: order_status_from(&column::<String>(&row, "status")?)?,
        };

        Ok(order.validate()?)
    }

    /// Without an id, inserts and the database decides it; with one, updates.
    async fn save(&self, order: Order) -> Result<OrderId, Error> {
        let Some(id) = order.id().cloned() else {
            let insert = sqlx::query(
                "INSERT INTO orders (product, quantity, total, status)
             VALUES ($1, $2, $3, $4) RETURNING id",
            )
            .bind(order.product.clone())
            .bind(order.quantity as i64)
            .bind(order.total as i64)
            .bind(order_status_name(order.status));

            let row = self.database.fetch_one(insert).await?;

            return Ok(OrderId::new(column::<i64>(&row, "id")? as u64)?);
        };

        let update = sqlx::query(
            "UPDATE orders SET product = $2, quantity = $3, total = $4, status = $5
             WHERE id = $1",
        )
        .bind(u64::from(id.clone()) as i64)
        .bind(order.product.clone())
        .bind(order.quantity as i64)
        .bind(order.total as i64)
        .bind(order_status_name(order.status));

        self.database.execute(update).await?;

        Ok(id)
    }
}

fn order_status_name(status: OrderStatus) -> &'static str {
    match status {
        OrderStatus::Placed => "Placed",
        OrderStatus::Paid => "Paid",
    }
}

fn order_status_from(name: &str) -> Result<OrderStatus, Error> {
    match name {
        "Placed" => Ok(OrderStatus::Placed),
        "Paid" => Ok(OrderStatus::Paid),
        _ => Err(InfrastructureError::from(anyhow::anyhow!("unknown OrderStatus {name}")))?,
    }
}
