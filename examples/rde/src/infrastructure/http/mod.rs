pub mod eth;
pub mod rpc;

use crate::ports::Ports;
use axum::Router;
use axum::routing::post;
use std::sync::Arc;

/// The HTTP API of the rde: JSON-RPC 2.0 on `POST /rpc`.
pub fn router(ports: Arc<Ports>) -> Router {
    Router::new()
        .route("/rpc", post(rpc::rpc))
        .with_state(ports)
}
