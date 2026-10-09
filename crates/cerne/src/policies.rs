use crate::commands::Command;
use crate::errors::{Error, InfrastructureError};
use crate::outbox::{OutboxEntry, command_name};
use serde::Serialize;

type Then<CompositionRoot> =
    Box<dyn FnOnce() -> (Box<dyn Command<CompositionRoot, Output = ()>>, Result<OutboxEntry, String>) + Send + Sync>;

/// The lilac post-it: "whenever `when` holds, run the command `then` returns".
///
/// The policy never runs the command itself: it only says which one should run, so the domain stays free of IO.
/// The command has `Output = ()`: nobody is there to receive what a command run by a policy gives back.
/// It derives `Serialize` (and `Deserialize`, to be read back), so an outbox can store it until it runs.
///
/// Written with [`policy!`](crate::domain::policy) and fired with [`Policies::trigger`].
pub struct Policy<CompositionRoot> {
    name: &'static str,
    when: Box<dyn Fn() -> bool + Send + Sync>,
    then: Then<CompositionRoot>,
}

impl<CompositionRoot: 'static> Policy<CompositionRoot> {
    pub fn new<C>(
        name: &'static str,
        when: impl Fn() -> bool + Send + Sync + 'static,
        then: impl FnOnce() -> Box<C> + Send + Sync + 'static,
    ) -> Self
    where
        C: Command<CompositionRoot, Output = ()> + Serialize + 'static,
    {
        let then = move || {
            let command = then();

            let outbox_entry = serde_json::to_string(&command)
                .map(|json| OutboxEntry {
                    command: command_name::<C>(),
                    json,
                })
                .map_err(|error| format!("cannot store {} in the outbox: {error}", command_name::<C>()));

            (command as Box<dyn Command<CompositionRoot, Output = ()>>, outbox_entry)
        };

        Self {
            name,
            when: Box::new(when),
            then: Box::new(then),
        }
    }
}

/// A policy that fired, with the command it wants to run.
pub struct FiredPolicy<CompositionRoot> {
    pub name: &'static str,
    pub command: Box<dyn Command<CompositionRoot, Output = ()>>,
    outbox_entry: Result<OutboxEntry, String>,
}

impl<CompositionRoot> FiredPolicy<CompositionRoot> {
    /// The same command, as the outbox stores it: its name and its fields in JSON.
    pub fn outbox_entry(&self) -> Result<OutboxEntry, Error> {
        let outbox_entry = self
            .outbox_entry
            .clone()
            .map_err(|error| InfrastructureError::from(anyhow::anyhow!(error)))?;

        Ok(outbox_entry)
    }
}

/// Fires the policies of a domain event: `Policies::trigger([policy!(..), ..])`.
pub struct Policies;

impl Policies {
    /// Fires every policy whose `when` holds; returns each one with the command its `then` built.
    pub fn trigger<CompositionRoot>(
        policies: impl IntoIterator<Item = Policy<CompositionRoot>>,
    ) -> Vec<FiredPolicy<CompositionRoot>> {
        policies
            .into_iter()
            .filter(|policy| (policy.when)())
            .map(|policy| {
                let (command, outbox_entry) = (policy.then)();

                FiredPolicy {
                    name: policy.name,
                    command,
                    outbox_entry,
                }
            })
            .collect()
    }
}

/// A [`Policy`](crate::domain::Policy): `policy!("whenever an order is placed, charge the customer", true, command)`.
///
/// Always three arguments: the name, as written on the board; the condition, a `bool` read once, right away (`true`
/// for a policy that always fires); and the command it sends, written as is. The macro writes the closures, and the
/// command is only built if the policy fires. It takes what it uses, so a value two policies need is cloned for one.
///
/// ```
/// # use cerne::application::{Command, Executed};
/// # use cerne::{Error, async_trait};
/// # use serde::{Deserialize, Serialize};
/// # struct CompositionRoot;
/// # #[derive(Serialize, Deserialize)]
/// # struct ChargeOrderCommand { order_id: u64, total: u64 }
/// # #[async_trait]
/// # impl Command<CompositionRoot> for ChargeOrderCommand {
/// #     type Output = ();
/// #     async fn execute(&self, _: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
/// #         Ok(Executed { output: (), events: vec![] })
/// #     }
/// # }
/// use cerne::domain::{Policies, policy};
///
/// let order_id = 7;
/// let total = 1500;
/// let order_is_large = total > 1000;
///
/// let charge_the_customer_policy = policy!(
///     "whenever an order is placed, charge the customer",
///     true,
///     ChargeOrderCommand { order_id, total }
/// );
/// let review_the_order_policy = policy!("whenever a large order is placed, review it", order_is_large, ChargeOrderCommand { order_id, total });
///
/// let fired = Policies::trigger([charge_the_customer_policy, review_the_order_policy]);
///
/// assert_eq!(fired.len(), 2);
/// # let _: &CompositionRoot = &CompositionRoot;
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! policy {
    ($name:expr, $when:expr, $then:expr $(,)?) => {{
        let when: bool = $when;

        $crate::domain::Policy::new($name, move || when, move || ::std::boxed::Box::new($then))
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::Executed;
    use async_trait::async_trait;
    use serde::Deserialize;
    use std::sync::Mutex;

    #[derive(Default)]
    struct CompositionRoot {
        shipped: Mutex<Vec<u64>>,
    }

    #[derive(Serialize, Deserialize)]
    struct ShipOrderCommand {
        order_id: u64,
    }

    #[async_trait]
    impl Command<CompositionRoot> for ShipOrderCommand {
        type Output = ();

        async fn execute(&self, composition_root: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
            composition_root.shipped.lock().unwrap().push(self.order_id);

            Ok(Executed {
                output: (),
                events: vec![],
            })
        }
    }

    #[test]
    fn trigger_fires_only_policies_whose_when_holds() {
        let is_admin = true;
        let never = false;

        let send_welcome_email_policy = policy!("send welcome email", true, ShipOrderCommand { order_id: 1 });
        let request_admin_approval_policy =
            policy!("request admin approval", is_admin, ShipOrderCommand { order_id: 2 });
        let never_policy = policy!("never", never, ShipOrderCommand { order_id: 3 });

        let fired = Policies::trigger([send_welcome_email_policy, request_admin_approval_policy, never_policy]);
        let fired_names: Vec<_> = fired
            .into_iter()
            .map(|fired_policy| fired_policy.name)
            .collect();

        assert_eq!(fired_names, vec!["send welcome email", "request admin approval"]);
    }

    #[tokio::test]
    async fn trigger_returns_the_command_built_by_then() {
        let composition_root = CompositionRoot::default();
        let order_id = 42;

        let ship_order_policy = policy!("ship order", true, ShipOrderCommand { order_id });

        for fired_policy in Policies::trigger([ship_order_policy]) {
            fired_policy
                .command
                .execute(&composition_root)
                .await
                .unwrap();
        }

        assert_eq!(*composition_root.shipped.lock().unwrap(), vec![42]);
    }

    #[test]
    fn fired_policy_carries_the_command_as_the_outbox_stores_it() {
        let ship_order_policy = policy!("ship order", true, ShipOrderCommand { order_id: 42 });

        let fired = Policies::trigger([ship_order_policy]);

        assert_eq!(
            fired[0].outbox_entry().unwrap(),
            OutboxEntry {
                command: "ship_order".into(),
                json: r#"{"order_id":42}"#.into()
            }
        );
    }

    #[test]
    fn the_command_of_a_policy_that_does_not_fire_is_never_built() {
        let never = false;

        let never_policy: Policy<CompositionRoot> =
            policy!("never", never, ship_order_command_that_must_not_be_built());

        assert!(Policies::trigger([never_policy]).is_empty());
    }

    fn ship_order_command_that_must_not_be_built() -> ShipOrderCommand {
        panic!("the command of a policy that did not fire was built")
    }
}
