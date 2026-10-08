use crate::application::commands::place_order::PlaceOrderCommand;
use crate::ports::Ports;
use axum::Json;
use axum::extract::State;
use cerne::Error;
use cerne::application::{Command, TransactionalPorts};
use std::sync::Arc;

/// `POST /orders`: the body is the `PlaceOrderCommand`, executed in a transaction; the answer is its output.
pub async fn place_order(
    State(ports): State<Arc<Ports>>,
    Json(place_order): Json<PlaceOrderCommand>,
) -> Result<Json<<PlaceOrderCommand as Command<Ports>>::Output>, Error> {
    ports.execute_in_transaction(place_order).await.map(Json)
}
