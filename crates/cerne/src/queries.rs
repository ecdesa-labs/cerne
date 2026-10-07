use crate::errors::Error;
use async_trait::async_trait;

/// The green post-it: what the actor looks at before deciding. A query only reads: it never changes state and
/// produces no events.
///
/// `Ports` is the same composition root the commands use. `ReadModel` is what the actor sees, shaped for the screen,
/// not like the aggregate.
///
/// ```
/// use cerne::application::{Query, ReadModel};
/// use cerne::{Error, async_trait};
///
/// struct Ports {
///     stock: Vec<(&'static str, i32)>,
/// }
///
/// /// What the buyer sees on the product page.
/// #[derive(Debug, PartialEq)]
/// struct ProductAvailability {
///     sku: &'static str,
///     in_stock: bool,
/// }
///
/// impl ReadModel for ProductAvailability {}
///
/// struct ProductAvailabilityQuery {
///     sku: &'static str,
/// }
///
/// #[async_trait]
/// impl Query<Ports> for ProductAvailabilityQuery {
///     type ReadModel = ProductAvailability;
///
///     async fn execute(&self, ports: &Ports) -> Result<ProductAvailability, Error> {
///         // --- Ports -----------------------------------------------------------
///
///         let available = ports.stock.iter().find(|(sku, _)| *sku == self.sku).map_or(0, |(_, units)| *units);
///
///         // --- Read model ------------------------------------------------------
///
///         let product_availability = ProductAvailability { sku: self.sku, in_stock: available > 0 };
///
///         Ok(product_availability)
///     }
/// }
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
/// let ports = Ports { stock: vec![("book", 3)] };
///
/// let product_availability = ProductAvailabilityQuery { sku: "book" }.execute(&ports).await?;
///
/// assert_eq!(product_availability, ProductAvailability { sku: "book", in_stock: true });
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait Query<Ports>: Send + Sync {
    type ReadModel: ReadModel;

    /// Reads the ports and builds the read model; changes nothing.
    async fn execute(&self, ports: &Ports) -> Result<Self::ReadModel, Error>;
}

/// The data a query gives back: plain fields, no behavior, no invariants.
pub trait ReadModel: Send {}
