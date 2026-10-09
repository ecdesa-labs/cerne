use crate::application::commands::place_order::PlaceOrderCommand;
use crate::composition_root::CompositionRoot;
use axum::Json;
use axum::extract::State;
use cerne::Error;
use cerne::application::{Command, TransactionalCompositionRoot};
use std::sync::Arc;

/// `POST /orders`: the body is the `PlaceOrderCommand`, executed in a transaction; the answer is its output.
pub async fn place_order(
    State(composition_root): State<Arc<CompositionRoot>>,
    Json(place_order): Json<PlaceOrderCommand>,
) -> Result<Json<<PlaceOrderCommand as Command<CompositionRoot>>::Output>, Error> {
    composition_root
        .execute_in_transaction(place_order)
        .await
        .map(Json)
}
