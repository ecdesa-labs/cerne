//! One block per flow of the board: an actor sends a command, and the test checks its events and the policies that fired.

use cerne::Error;
use cerne::application::{Command, Query, SyncEventBus};
use cerne::domain::{DomainError, ValueObject};
use shop::application::commands::place_order::PlaceOrderCommand;
use shop::application::queries::order_summary::OrderSummaryQuery;
use shop::application::read_models::order_summary::OrderSummary;
use shop::composition_root::{CompositionRoot, CompositionRootConstructor};
use shop::domain::value_objects::order_id::OrderId;
use shop::infrastructure::in_memory_catalog::InMemoryCatalog;
use shop::infrastructure::in_memory_database::{InMemoryDatabase, InMemoryEventOutbox, InMemoryOrderRepository};
use shop::infrastructure::in_memory_payments::InMemoryPayments;
use std::sync::Arc;

fn composition_root(payments: Arc<InMemoryPayments>) -> Arc<CompositionRoot> {
    let database = InMemoryDatabase::default();

    let order_repository = InMemoryOrderRepository::new(database.clone());
    let event_outbox = InMemoryEventOutbox::new(database.clone());

    let catalog = InMemoryCatalog {
        products: vec![("mug", 3000, 10)],
    };

    let composition_root_constructor = CompositionRootConstructor {
        database,
        order_repository: Box::new(order_repository),
        event_outbox: Box::new(event_outbox),
        catalog: Arc::new(catalog),
        payments,
    };

    Arc::new(CompositionRoot::new(composition_root_constructor))
}

#[tokio::test]
async fn the_customer_places_an_order_and_the_policy_charges_it() -> anyhow::Result<()> {
    let payments = Arc::new(InMemoryPayments::default());
    let composition_root = composition_root(Arc::clone(&payments));

    let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), |error| eprintln!("policy: {error}"));

    // --- Customer: places an order -------------------------------------------

    let place_order = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 2,
    };

    let place_order_execution = place_order.execute(&composition_root).await?;

    let order_id = place_order_execution.output;

    sync_event_bus.publish(place_order_execution.events).await;

    // --- Policy: whenever an order is placed, charge the customer -----------

    assert_eq!(*payments.charges.lock().unwrap(), vec![(order_id.clone(), 6000)]);

    let stored_events: Vec<String> = composition_root
        .database
        .event_outbox
        .lock()
        .unwrap()
        .iter()
        .map(|outbox_entry| outbox_entry.event.clone())
        .collect();

    assert_eq!(stored_events, ["order_placed", "order_paid"]);

    // --- Customer: reads the order -------------------------------------------

    let order_summary_query = OrderSummaryQuery { order_id };

    let order_summary = order_summary_query.execute(&composition_root).await?;

    let paid_order_summary = OrderSummary {
        product: "mug".into(),
        quantity: 2,
        total: 6000,
        status: "Paid".into(),
    };

    assert_eq!(order_summary, paid_order_summary);

    Ok(())
}

#[tokio::test]
async fn the_domain_refuses_what_breaks_a_rule_or_an_invariant() -> anyhow::Result<()> {
    let composition_root = composition_root(Arc::default());

    // --- Business rule: stock covers the quantity ----------------------------

    let too_many_mugs = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 11,
    };

    let refused = too_many_mugs.execute(&composition_root).await;

    assert!(matches!(
        refused,
        Err(Error::Domain(DomainError::Violations(violations))) if violations == ["stock covers the quantity"]
    ));

    // --- Invariant: quantity is positive -------------------------------------

    let no_mugs = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 0,
    };

    let refused = no_mugs.execute(&composition_root).await;

    assert!(matches!(
        refused,
        Err(Error::Domain(DomainError::Violations(violations))) if violations == ["quantity is positive"]
    ));

    // --- Value object: order id is positive ----------------------------------

    assert!(OrderId::new(0).is_err());

    Ok(())
}
