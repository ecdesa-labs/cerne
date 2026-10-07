use crate::commands::Command;
use crate::errors::{Error, InfrastructureError};
use crate::outbox::{OutboxEntry, command_name};
use serde::Serialize;

type Then<Ports> = Box<
    dyn Fn() -> (
            Box<dyn Command<Ports, Output = ()>>,
            Result<OutboxEntry, String>,
        ) + Send
        + Sync,
>;

/// The lilac post-it: "whenever `when` holds, run the command `then` returns".
///
/// The policy never runs the command itself: it only says which one should run, so the domain stays free of IO.
/// The command has `Output = ()`: nobody is there to receive what a command run by a policy gives back.
/// It derives `Serialize` (and `Deserialize`, to be read back), so an outbox can store it until it runs (D32).
pub struct Policy<Ports> {
    name: &'static str,
    when: Box<dyn Fn() -> bool + Send + Sync>,
    then: Then<Ports>,
}

impl<Ports: 'static> Policy<Ports> {
    pub fn new<C>(
        name: &'static str,
        when: impl Fn() -> bool + Send + Sync + 'static,
        then: impl Fn() -> Box<C> + Send + Sync + 'static,
    ) -> Self
    where
        C: Command<Ports, Output = ()> + Serialize + 'static,
    {
        let then = move || {
            let command = then();

            let outbox_entry = serde_json::to_string(&command)
                .map(|json| OutboxEntry {
                    command: command_name::<C>(),
                    json,
                })
                .map_err(|error| {
                    format!(
                        "cannot store {} in the outbox: {error}",
                        command_name::<C>()
                    )
                });

            (
                command as Box<dyn Command<Ports, Output = ()>>,
                outbox_entry,
            )
        };

        Self {
            name,
            when: Box::new(when),
            then: Box::new(then),
        }
    }
}

/// A policy that fired, with the command it wants to run.
pub struct FiredPolicy<Ports> {
    pub name: &'static str,
    pub command: Box<dyn Command<Ports, Output = ()>>,
    outbox_entry: Result<OutboxEntry, String>,
}

impl<Ports> FiredPolicy<Ports> {
    /// The same command, as the outbox stores it: its name and its fields in JSON.
    pub fn outbox_entry(&self) -> Result<OutboxEntry, Error> {
        let outbox_entry = self
            .outbox_entry
            .clone()
            .map_err(|error| InfrastructureError::from(anyhow::anyhow!(error)))?;

        Ok(outbox_entry)
    }
}

pub struct Policies<Ports>(Vec<Policy<Ports>>);

impl<Ports> Policies<Ports> {
    pub fn new(policies: Vec<Policy<Ports>>) -> Self {
        Self(policies)
    }

    /// Fires every policy whose `when` holds; returns each one with the command its `then` built.
    pub fn trigger(&self) -> Vec<FiredPolicy<Ports>> {
        self.0
            .iter()
            .filter(|p| (p.when)())
            .map(|p| {
                let (command, outbox_entry) = (p.then)();

                FiredPolicy {
                    name: p.name,
                    command,
                    outbox_entry,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::Executed;
    use async_trait::async_trait;
    use serde::Deserialize;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Ports {
        shipped: Mutex<Vec<u64>>,
    }

    #[derive(Serialize, Deserialize)]
    struct ShipOrderCommand {
        order_id: u64,
    }

    #[async_trait]
    impl Command<Ports> for ShipOrderCommand {
        type Output = ();

        async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
            ports.shipped.lock().unwrap().push(self.order_id);

            Ok(Executed {
                output: (),
                events: vec![],
            })
        }
    }

    fn ship(order_id: u64) -> Box<ShipOrderCommand> {
        Box::new(ShipOrderCommand { order_id })
    }

    #[test]
    fn trigger_fires_only_policies_whose_when_holds() {
        let is_admin = true;

        let policies = Policies::new(vec![
            Policy::new("send welcome email", || true, || ship(1)),
            Policy::new("request admin approval", move || is_admin, || ship(2)),
            Policy::new("never", || false, || ship(3)),
        ]);

        let fired: Vec<_> = policies.trigger().into_iter().map(|f| f.name).collect();

        assert_eq!(fired, vec!["send welcome email", "request admin approval"]);
    }

    #[tokio::test]
    async fn trigger_returns_the_command_built_by_then() {
        let ports = Ports::default();
        let order_id = 42;

        let policies = Policies::new(vec![Policy::new(
            "ship order",
            || true,
            move || ship(order_id),
        )]);

        for fired in policies.trigger() {
            fired.command.execute(&ports).await.unwrap();
        }

        assert_eq!(*ports.shipped.lock().unwrap(), vec![42]);
    }

    #[test]
    fn fired_policy_carries_the_command_as_the_outbox_stores_it() {
        let policies = Policies::new(vec![Policy::new("ship order", || true, || ship(42))]);

        let fired = policies.trigger();

        assert_eq!(
            fired[0].outbox_entry().unwrap(),
            OutboxEntry {
                command: "ship_order".into(),
                json: r#"{"order_id":42}"#.into(),
            }
        );
    }
}
