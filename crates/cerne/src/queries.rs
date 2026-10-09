use crate::errors::Error;
use async_trait::async_trait;

/// The green post-it: what the actor looks at before deciding. A query only reads: it never changes state and
/// produces no events.
///
/// `CompositionRoot` is the same composition root the commands use. `ReadModel` is what the actor sees: a struct of plain
/// fields, no behavior and no invariants, shaped for the screen, not like the aggregate.
///
/// ```
/// use cerne::application::Query;
/// use cerne::{Error, async_trait};
///
/// struct CompositionRoot {
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
/// struct ProductAvailabilityQuery {
///     sku: &'static str,
/// }
///
/// #[async_trait]
/// impl Query<CompositionRoot> for ProductAvailabilityQuery {
///     type ReadModel = ProductAvailability;
///
///     async fn execute(&self, composition_root: &CompositionRoot) -> Result<ProductAvailability, Error> {
///         // --- Ports -----------------------------------------------------------
///
///         let available = composition_root.stock.iter().find(|(sku, _)| *sku == self.sku).map_or(0, |(_, units)| *units);
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
/// let composition_root = CompositionRoot { stock: vec![("book", 3)] };
///
/// let product_availability = ProductAvailabilityQuery { sku: "book" }.execute(&composition_root).await?;
///
/// assert_eq!(product_availability, ProductAvailability { sku: "book", in_stock: true });
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait Query<CompositionRoot>: Send + Sync {
    type ReadModel: Send;

    /// Reads the ports and builds the read model; changes nothing.
    async fn execute(&self, composition_root: &CompositionRoot) -> Result<Self::ReadModel, Error>;
}
