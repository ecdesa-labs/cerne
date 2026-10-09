use crate::application::queries::order_summary::OrderSummaryQuery;
use crate::application::read_models::order_summary::OrderSummary;
use crate::composition_root::CompositionRoot;
use axum::Json;
use axum::extract::State;
use cerne::Error;
use cerne::application::Query;
use std::sync::Arc;

/// `GET /orders`: the query string is the `OrderSummaryQuery`; the answer is the `OrderSummary` read model.
pub async fn order_summary(
    State(composition_root): State<Arc<CompositionRoot>>,
    axum::extract::Query(order_summary): axum::extract::Query<OrderSummaryQuery>,
) -> Result<Json<OrderSummary>, Error> {
    let read_model = order_summary.execute(composition_root.as_ref()).await?;

    Ok(Json(read_model))
}
