use crate::ports::Ports;
use axum::Router;
use std::sync::Arc;

/// The HTTP API: one route per endpoint, added by `cerne g endpoint`.
pub fn router(ports: Arc<Ports>) -> Router {
    Router::new()
        .route("/orders", axum::routing::post(place_order::place_order))
        .route("/orders", axum::routing::get(order_summary::order_summary))
        .with_state(ports)
}
pub mod order_summary;
pub mod place_order;
