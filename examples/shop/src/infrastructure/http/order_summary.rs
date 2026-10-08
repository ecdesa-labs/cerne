use crate::application::queries::order_summary::OrderSummaryQuery;
use crate::application::read_models::order_summary::OrderSummary;
use crate::ports::Ports;
use axum::Json;
use axum::extract::State;
use cerne::Error;
use cerne::application::Query;
use std::sync::Arc;

/// `GET /orders`: the query string is the `OrderSummaryQuery`; the answer is the `OrderSummary` read model.
pub async fn order_summary(
    State(ports): State<Arc<Ports>>,
    axum::extract::Query(order_summary): axum::extract::Query<OrderSummaryQuery>,
) -> Result<Json<OrderSummary>, Error> {
    let read_model = order_summary.execute(ports.as_ref()).await?;

    Ok(Json(read_model))
}
