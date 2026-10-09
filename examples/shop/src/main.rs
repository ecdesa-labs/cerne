use cerne::application::{Command, Query, SyncEventBus};
use shop::application::commands::place_order::PlaceOrderCommand;
use shop::application::queries::order_summary::OrderSummaryQuery;
use shop::composition_root::{CompositionRoot, CompositionRootConstructor};
use shop::infrastructure::in_memory_catalog::InMemoryCatalog;
use shop::infrastructure::in_memory_database::{InMemoryDatabase, InMemoryEventOutbox, InMemoryOrderRepository};
use shop::infrastructure::in_memory_payments::InMemoryPayments;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Composition root ----------------------------------------------------

    let database = InMemoryDatabase::default();

    let order_repository = InMemoryOrderRepository::new(database.clone());
    let event_outbox = InMemoryEventOutbox::new(database.clone());

    let catalog = InMemoryCatalog {
        products: vec![("mug", 3000, 10), ("t-shirt", 5000, 3)],
    };
    let payments = InMemoryPayments::default();

    let composition_root_constructor = CompositionRootConstructor {
        database,
        order_repository: Box::new(order_repository),
        event_outbox: Box::new(event_outbox),
        catalog: Arc::new(catalog),
        payments: Arc::new(payments),
    };

    let composition_root = Arc::new(CompositionRoot::new(composition_root_constructor));

    // --- Event bus: the policies of every event ------------------------------

    let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), |error| eprintln!("policy: {error}"));

    // --- Customer: places an order -------------------------------------------

    let place_order = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 2,
    };

    let place_order_execution = place_order.execute(&composition_root).await?;

    let order_id = place_order_execution.output;

    sync_event_bus.publish(place_order_execution.events).await;

    // --- Customer: reads the order -------------------------------------------

    let order_summary_query = OrderSummaryQuery { order_id };

    let order_summary = order_summary_query.execute(&composition_root).await?;

    println!("{order_summary:?}");

    Ok(())
}
