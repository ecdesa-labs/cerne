use cerne::application::{Command, EventOutbox, Query, Repository, SequentialEventBus};
use shop::application::commands::place_order::PlaceOrderCommand;
use shop::application::queries::order_summary::OrderSummaryQuery;
use shop::composition_root::{CompositionRoot, CompositionRootConstructor};
use shop::domain::entities::order::Order;
use shop::infrastructure::database::{Database, Transaction};
use shop::infrastructure::http_catalog::HttpCatalog;
use shop::infrastructure::http_payments::HttpPayments;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Composition root ----------------------------------------------------

    let database = Database;

    let order_repository = order_repository_adapter();
    let event_outbox = event_outbox_adapter();

    let catalog = HttpCatalog;
    let payments = HttpPayments;

    let composition_root_constructor = CompositionRootConstructor {
        database,
        order_repository,
        event_outbox,
        catalog: Arc::new(catalog),
        payments: Arc::new(payments),
    };

    let composition_root = Arc::new(CompositionRoot::new(composition_root_constructor));

    // --- Event bus: the policies of every event ------------------------------

    let sequential_event_bus =
        SequentialEventBus::new(Arc::clone(&composition_root), |error| eprintln!("policy: {error}"));

    // --- Customer: places an order -------------------------------------------

    let place_order = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 2,
    };

    let place_order_execution = place_order.execute(&composition_root).await?;

    let order_id = place_order_execution.output;

    sequential_event_bus
        .publish(place_order_execution.events)
        .await;

    // --- Customer: reads the order -------------------------------------------

    let order_summary_query = OrderSummaryQuery { order_id };

    let order_summary = order_summary_query.execute(&composition_root).await?;

    println!("{order_summary:?}");

    Ok(())
}

/// No adapter of EventOutbox yet: write one in `infrastructure/` and build it here.
fn event_outbox_adapter() -> Box<dyn EventOutbox<Transaction>> {
    todo!("an adapter of EventOutbox")
}

/// No adapter of Repository<Order> yet: write one in `infrastructure/` and build it here.
fn order_repository_adapter() -> Box<dyn Repository<Order, Transaction>> {
    todo!("an adapter of Repository<Order>")
}
