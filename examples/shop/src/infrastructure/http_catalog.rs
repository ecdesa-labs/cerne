use crate::application::ports::catalog::Catalog;
use cerne::{Error, async_trait};

/// The client of the catalog's HTTP API: writing it is yours (with `reqwest` or any other).
pub struct HttpCatalog;

#[async_trait]
impl Catalog for HttpCatalog {
    async fn unit_price(&self, _product: &str) -> Result<u64, Error> {
        todo!("ask the catalog API for the unit price")
    }

    async fn units_in_stock(&self, _product: &str) -> Result<u32, Error> {
        todo!("ask the catalog API for the units in stock")
    }
}
