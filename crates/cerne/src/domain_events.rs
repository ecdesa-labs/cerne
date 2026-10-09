use crate::errors::EnforcementResult;
use crate::policies::FiredPolicy;

/// The orange post-it: something that happened in the domain, and the policies it sets off.
///
/// `CompositionRoot` is the application the commands of those policies run on.
///
/// ```
/// use cerne::application::{Command, Executed};
/// use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, policy};
/// use cerne::{Error, async_trait};
///
/// struct CompositionRoot;
///
/// struct ReserveStockCommand {
///     order_id: u64,
/// }
///
/// #[async_trait]
/// impl Command<CompositionRoot> for ReserveStockCommand {
///     type Output = ();
///
///     async fn execute(&self, _ports: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
///         Ok(Executed { output: (), events: vec![] })
///     }
/// }
///
/// struct OrderPlaced {
///     order_id: u64,
/// }
///
/// impl DomainEvent<CompositionRoot> for OrderPlaced {
///     fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> {
///         let order_id = self.order_id;
///
///         let reserve_stock_policy = policy!("reserve stock", true, ReserveStockCommand { order_id });
///
///         Ok(Policies::trigger([reserve_stock_policy]))
///     }
/// }
///
/// let fired = OrderPlaced { order_id: 1 }.trigger_policies().unwrap();
///
/// assert_eq!(fired[0].name, "reserve stock");
/// ```
pub trait DomainEvent<CompositionRoot>: Send + Sync {
    /// A domain event only triggers its policies if its invariants hold; returns those that fired.
    fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Command, Executed};
    use crate::errors::{DomainError, Error};
    use crate::invariant;
    use crate::invariants::Invariants;
    use crate::policies::Policies;
    use crate::policy;
    use async_trait::async_trait;

    struct CompositionRoot;

    struct NoopCommand;

    #[async_trait]
    impl Command<CompositionRoot> for NoopCommand {
        type Output = ();

        async fn execute(&self, _: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
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

    impl DomainEvent<CompositionRoot> for OrderPlaced {
        fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> {
            let has_id = !self.id.is_empty();
            let has_items = !self.items.is_empty();
            let is_large = self.items.len() > 10;

            Invariants::enforce([invariant!("order has an id", has_id), invariant!("order has items", has_items)])?;

            let reserve_stock_policy = policy!("reserve stock", true, NoopCommand);
            let request_manual_review_policy = policy!("request manual review", is_large, NoopCommand);

            Ok(Policies::trigger([reserve_stock_policy, request_manual_review_policy]))
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

        assert_eq!(event.fired(), Ok(vec!["reserve stock", "request manual review"]));
    }

    #[test]
    fn invalid_event_triggers_nothing() {
        let event = OrderPlaced::new("", vec![]);

        assert_eq!(event.fired(), Err(DomainError::Violations(vec!["order has an id", "order has items"])));
    }
}
