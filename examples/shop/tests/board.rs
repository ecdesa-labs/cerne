//! One block per flow of the board: an actor sends a command, and the test checks its events and the policies that fired.

use cerne::domain::{DomainError, DomainEvent, Entity, ValueObject};
use shop::domain::entities::order::{Order, OrderConstructor, OrderStatus};
use shop::domain::events::order_placed::OrderPlaced;
use shop::domain::value_objects::order_id::OrderId;

#[test]
fn the_customer_places_an_order_and_the_policy_charges_it() -> anyhow::Result<()> {
    // --- Customer: places an order -------------------------------------------

    let order = Order::new(OrderConstructor {
        product: "mug".into(),
        quantity: 2,
        total: 6000,
    })?;

    assert_eq!(order.status, OrderStatus::Placed);

    // --- Policy: whenever an order is placed, charge the customer -----------

    let order_placed = OrderPlaced {
        order_id: OrderId::new(1)?,
        total: 6000,
    };

    let fired_policies = order_placed.trigger_policies()?;
    let fired_policy_names: Vec<&str> = fired_policies
        .iter()
        .map(|fired_policy| fired_policy.name)
        .collect();

    assert_eq!(fired_policy_names, ["whenever an order is placed, charge the customer"]);

    // --- Payments: the order is paid -----------------------------------------

    let paid_order = order.pay()?;

    assert_eq!(paid_order.status, OrderStatus::Paid);

    Ok(())
}

#[test]
fn the_domain_refuses_what_breaks_an_invariant() {
    // --- Invariant: quantity is positive -------------------------------------

    let no_mugs = Order::new(OrderConstructor {
        product: "mug".into(),
        quantity: 0,
        total: 0,
    });

    assert!(matches!(no_mugs, Err(DomainError::Violations(violations)) if violations == ["quantity is positive"]));

    // --- Value object: order id is positive ----------------------------------

    assert!(OrderId::new(0).is_err());
}
