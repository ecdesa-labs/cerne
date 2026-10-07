use crate::errors::EnforcementResult;
use crate::policies::FiredPolicy;

/// The orange post-it: something that happened in the domain, and the policies it sets off.
///
/// `Ports` is the application the commands of those policies run on.
///
/// ```
/// use cerne::application::{Command, Executed};
/// use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, Policy};
/// use cerne::{Error, async_trait};
///
/// struct Ports;
///
/// struct ReserveStockCommand {
///     order_id: u64,
/// }
///
/// #[async_trait]
/// impl Command<Ports> for ReserveStockCommand {
///     type Output = ();
///
///     async fn execute(&self, _ports: &Ports) -> Result<Executed<(), Ports>, Error> {
///         Ok(Executed { output: (), events: vec![] })
///     }
/// }
///
/// struct OrderPlaced {
///     order_id: u64,
/// }
///
/// impl DomainEvent<Ports> for OrderPlaced {
///     fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
///         let order_id = self.order_id;
///
///         Ok(Policies::new(vec![Policy::new("reserve stock", || true, move || {
///             Box::new(ReserveStockCommand { order_id })
///         })])
///         .trigger())
///     }
/// }
///
/// let fired = OrderPlaced { order_id: 1 }.trigger_policies().unwrap();
///
/// assert_eq!(fired[0].name, "reserve stock");
/// ```
pub trait DomainEvent<Ports>: Send + Sync {
    /// A domain event only triggers its policies if its invariants hold; returns those that fired.
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Command, Executed};
    use crate::errors::{DomainError, Error};
    use crate::invariants::{Invariant, Invariants};
    use crate::policies::{Policies, Policy};
    use async_trait::async_trait;

    struct Ports;

    struct NoopCommand;

    #[async_trait]
    impl Command<Ports> for NoopCommand {
        type Output = ();

        async fn execute(&self, _: &Ports) -> Result<Executed<(), Ports>, Error> {
            Ok(Executed {
                output: (),
                events: vec![],
            })
        }
    }

    pub struct OrderPlaced {
        id: &'static str,
        items: Vec<&'static str>,
    }

    impl OrderPlaced {
        fn new(id: &'static str, items: Vec<&'static str>) -> Self {
            Self { id, items }
        }

        fn fired(&self) -> EnforcementResult<Vec<&'static str>> {
            Ok(self
                .trigger_policies()?
                .into_iter()
                .map(|f| f.name)
                .collect())
        }
    }

    impl DomainEvent<Ports> for OrderPlaced {
        fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
            let has_id = !self.id.is_empty();
            let has_items = !self.items.is_empty();
            let is_large = self.items.len() > 10;

            Invariants::new(vec![
                Invariant::new("order has an id", move || has_id),
                Invariant::new("order has items", move || has_items),
            ])
            .enforce()?;

            Ok(Policies::new(vec![
                Policy::new("reserve stock", || true, || Box::new(NoopCommand)),
                Policy::new(
                    "request manual review",
                    move || is_large,
                    || Box::new(NoopCommand),
                ),
            ])
            .trigger())
        }
    }

    #[test]
    fn event_triggers_its_policies() {
        let event = OrderPlaced::new("order-1", vec!["book"]);

        assert_eq!(event.fired(), Ok(vec!["reserve stock"]));
    }

    #[test]
    fn event_triggers_conditional_policy_when_it_holds() {
        let event = OrderPlaced::new("order-1", vec!["book"; 11]);

        assert_eq!(
            event.fired(),
            Ok(vec!["reserve stock", "request manual review"])
        );
    }

    #[test]
    fn invalid_event_triggers_nothing() {
        let event = OrderPlaced::new("", vec![]);

        assert_eq!(
            event.fired(),
            Err(DomainError::Violations(vec![
                "order has an id",
                "order has items"
            ]))
        );
    }
}
