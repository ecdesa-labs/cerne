//! One block per flow of the board: an actor sends a command, and the test checks its events and the policies that fired.

use cerne::Error;
use cerne::application::{OutboxPolicyProcessor, Query, TransactionalPorts};
use cerne::domain::{DomainError, ValueObject};
use cerne::sqlite::SqliteDatabase;
use shop::application::commands::place_order::PlaceOrderCommand;
use shop::application::queries::order_summary::OrderSummaryQuery;
use shop::application::read_models::order_summary::OrderSummary;
use shop::domain::value_objects::order_id::OrderId;
use shop::infrastructure::in_memory_catalog::InMemoryCatalog;
use shop::infrastructure::in_memory_payments::InMemoryPayments;
use shop::ports::{Ports, command_registry};
use std::sync::Arc;

async fn ports(payments: Arc<InMemoryPayments>) -> Result<Arc<Ports>, Error> {
    let database = SqliteDatabase::in_memory().await?;

    database.migrate(&sqlx::migrate!()).await?;

    let catalog = Arc::new(InMemoryCatalog {
        products: vec![("mug", 3000, 10)],
    });

    Ok(Arc::new(Ports::new(database, catalog, payments)))
}

#[tokio::test]
async fn the_customer_places_an_order_and_the_policy_charges_it() -> Result<(), Error> {
    let payments = Arc::new(InMemoryPayments::default());
    let ports = ports(Arc::clone(&payments)).await?;

    let outbox_policy_processor =
        OutboxPolicyProcessor::new(Arc::clone(&ports), command_registry());

    // --- Customer: places an order -------------------------------------------

    let place_order = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 2,
    };

    let order_id = ports.execute_in_transaction(place_order).await?;

    // --- Policy: whenever an order is placed, charge the customer -----------

    let command_runs = outbox_policy_processor.run_pending().await?;

    assert_eq!(command_runs.len(), 1);
    assert_eq!(
        *payments.charges.lock().unwrap(),
        vec![(order_id.clone(), 6000)]
    );

    // --- Customer: reads the order -------------------------------------------

    let order_summary_query = OrderSummaryQuery { order_id };

    let order_summary = order_summary_query.execute(&ports).await?;

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
async fn the_domain_refuses_what_breaks_a_rule_or_an_invariant() -> Result<(), Error> {
    let ports = ports(Arc::default()).await?;

    // --- Business rule: stock covers the quantity ----------------------------

    let too_many_mugs = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 11,
    };

    let refused = ports.execute_in_transaction(too_many_mugs).await;

    assert!(matches!(
        refused,
        Err(Error::Domain(DomainError::Violations(violations))) if violations == ["stock covers the quantity"]
    ));

    // --- Invariant: quantity is positive -------------------------------------

    let no_mugs = PlaceOrderCommand {
        product: "mug".into(),
        quantity: 0,
    };

    let refused = ports.execute_in_transaction(no_mugs).await;

    assert!(matches!(
        refused,
        Err(Error::Domain(DomainError::Violations(violations))) if violations == ["quantity is positive"]
    ));

    // --- Value object: order id is positive ----------------------------------

    assert!(OrderId::new(0).is_err());

    Ok(())
}
