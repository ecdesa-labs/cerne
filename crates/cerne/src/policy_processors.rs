use crate::commands::Command;
use crate::domain_events::DomainEvent;
use crate::errors::{Error, InfrastructureError};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Runs the commands that policies return, closing the loop Event → Policy → Command.
///
/// When those commands run (right away, in the background, from an outbox table) is up to the implementation.
#[async_trait]
pub trait PolicyProcessor<Ports: 'static>: Send + Sync {
    /// Runs the command, and then the commands its events trigger in turn.
    async fn send_command(
        &self,
        command: Box<dyn Command<Ports, Output = ()>>,
    ) -> Result<(), Error>;

    /// Triggers the policies of every event and sends the commands they return; returns the names of the policies that fired.
    ///
    /// If the invariants of any event fail, no command is sent.
    async fn send_events(
        &self,
        events: Vec<Box<dyn DomainEvent<Ports>>>,
    ) -> Result<Vec<&'static str>, Error> {
        let mut fired = vec![];
        for event in events {
            fired.extend(event.trigger_policies()?);
        }

        let mut names = vec![];
        for policy in fired {
            self.send_command(policy.command).await?;
            names.push(policy.name);
        }

        Ok(names)
    }
}

/// The default `PolicyProcessor`: runs the whole chain of policies right away, before `send_events` returns.
///
/// A failing command stops the chain, and its error comes back to the caller.
///
/// ```
/// # use cerne::application::{Command, Executed};
/// # use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, Policy};
/// # use cerne::{Error, async_trait};
/// # struct Ports;
/// # #[derive(serde::Serialize)]
/// # struct ReserveStockCommand;
/// # #[async_trait]
/// # impl Command<Ports> for ReserveStockCommand {
/// #     type Output = ();
/// #
/// #     async fn execute(&self, _: &Ports) -> Result<Executed<(), Ports>, Error> { Ok(Executed { output: (), events: vec![] }) }
/// # }
/// # struct OrderPlaced;
/// # impl DomainEvent<Ports> for OrderPlaced {
/// #     fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
/// #         Ok(Policies::new(vec![Policy::new("reserve stock", || true, || Box::new(ReserveStockCommand))]).trigger())
/// #     }
/// # }
/// use cerne::application::{InlinePolicyProcessor, PolicyProcessor};
/// use std::sync::Arc;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
/// let sync_policy_processor = InlinePolicyProcessor::new(Arc::new(Ports));
///
/// let place_order_events: Vec<Box<dyn DomainEvent<Ports>>> = vec![Box::new(OrderPlaced)];
/// let fired_policies = sync_policy_processor.send_events(place_order_events).await?; // ReserveStockCommand already ran
///
/// assert_eq!(fired_policies, vec!["reserve stock"]);
/// # Ok(())
/// # }
/// ```
pub struct InlinePolicyProcessor<Ports> {
    ports: Arc<Ports>,
}

impl<Ports> InlinePolicyProcessor<Ports> {
    pub fn new(ports: Arc<Ports>) -> Self {
        Self { ports }
    }
}

#[async_trait]
impl<Ports: Send + Sync + 'static> PolicyProcessor<Ports> for InlinePolicyProcessor<Ports> {
    async fn send_command(
        &self,
        command: Box<dyn Command<Ports, Output = ()>>,
    ) -> Result<(), Error> {
        let mut pending = VecDeque::from([command]);

        while let Some(command) = pending.pop_front() {
            pending.extend(run(command, &self.ports).await?);
        }

        Ok(())
    }
}

/// A `PolicyProcessor` for production: a `tokio` task runs the commands in the background, so `send_events` returns at once.
///
/// The channel lives in memory: if the process dies, the commands still waiting are lost.
///
/// ```
/// # use cerne::application::{Command, Executed};
/// # use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, Policy};
/// # use cerne::{Error, async_trait};
/// # struct Ports;
/// # #[derive(serde::Serialize)]
/// # struct ReserveStockCommand;
/// # #[async_trait]
/// # impl Command<Ports> for ReserveStockCommand {
/// #     type Output = ();
/// #
/// #     async fn execute(&self, _: &Ports) -> Result<Executed<(), Ports>, Error> { Ok(Executed { output: (), events: vec![] }) }
/// # }
/// # struct OrderPlaced;
/// # impl DomainEvent<Ports> for OrderPlaced {
/// #     fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
/// #         Ok(Policies::new(vec![Policy::new("reserve stock", || true, || Box::new(ReserveStockCommand))]).trigger())
/// #     }
/// # }
/// use cerne::application::{PolicyProcessor, TokioPolicyProcessor};
/// use std::sync::Arc;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
/// let async_policy_processor = TokioPolicyProcessor::spawn(Arc::new(Ports), |error| eprintln!("{error}"));
///
/// let place_order_events: Vec<Box<dyn DomainEvent<Ports>>> = vec![Box::new(OrderPlaced)];
/// async_policy_processor.send_events(place_order_events).await?; // ReserveStockCommand runs in the background
///
/// async_policy_processor.shutdown().await?; // waits for every command sent
/// # Ok(())
/// # }
/// ```
pub struct TokioPolicyProcessor<Ports> {
    commands: mpsc::UnboundedSender<Box<dyn Command<Ports, Output = ()>>>,
    task: JoinHandle<()>,
}

impl<Ports: Send + Sync + 'static> TokioPolicyProcessor<Ports> {
    /// Spawns the task. A failing command goes to `on_error`, and the task moves on to the next one.
    /// Must be called inside a `tokio` runtime.
    pub fn spawn(ports: Arc<Ports>, on_error: impl Fn(Error) + Send + 'static) -> Self {
        let (commands, mut received) =
            mpsc::unbounded_channel::<Box<dyn Command<Ports, Output = ()>>>();

        let task = tokio::spawn(async move {
            while let Some(command) = received.recv().await {
                let mut pending = VecDeque::from([command]);

                while let Some(command) = pending.pop_front() {
                    match run(command, &ports).await {
                        Ok(next) => pending.extend(next),
                        Err(error) => on_error(error),
                    }
                }
            }
        });

        Self { commands, task }
    }

    /// Stops accepting commands and waits until every command already sent has run.
    pub async fn shutdown(self) -> Result<(), Error> {
        drop(self.commands);

        self.task
            .await
            .map_err(|error| InfrastructureError::from(anyhow::Error::from(error)))?;

        Ok(())
    }
}

#[async_trait]
impl<Ports: Send + Sync + 'static> PolicyProcessor<Ports> for TokioPolicyProcessor<Ports> {
    async fn send_command(
        &self,
        command: Box<dyn Command<Ports, Output = ()>>,
    ) -> Result<(), Error> {
        self.commands
            .send(command)
            .map_err(|_| InfrastructureError::from(anyhow::anyhow!("policy processor stopped")))?;

        Ok(())
    }
}

/// Runs one command and returns the commands its events trigger.
async fn run<Ports>(
    command: Box<dyn Command<Ports, Output = ()>>,
    ports: &Ports,
) -> Result<Vec<Box<dyn Command<Ports, Output = ()>>>, Error> {
    let mut next = vec![];
    let execution = command.execute(ports).await?;

    for event in execution.events {
        next.extend(event.trigger_policies()?.into_iter().map(|p| p.command));
    }

    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::Executed;
    use crate::errors::{DomainError, EnforcementResult};
    use crate::invariants::{Invariant, Invariants};
    use crate::policies::{FiredPolicy, Policies, Policy};
    use std::sync::Mutex;

    #[derive(Default)]
    struct Ports {
        log: Mutex<Vec<&'static str>>,
    }

    /// Place order → OrderPlaced → "reserve stock" → reserve stock → StockReserved → "ship order" → ship order.
    struct PlaceOrderCommand;
    #[derive(serde::Serialize)]
    struct ReserveStockCommand;
    #[derive(serde::Serialize)]
    struct ShipOrderCommand;
    struct FailingCommand;

    struct OrderPlaced {
        has_items: bool,
    }
    struct StockReserved;

    #[async_trait]
    impl Command<Ports> for PlaceOrderCommand {
        type Output = ();

        async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
            ports.log.lock().unwrap().push("order placed");

            let order_placed = OrderPlaced { has_items: true };

            Ok(Executed {
                output: (),
                events: vec![Box::new(order_placed)],
            })
        }
    }

    #[async_trait]
    impl Command<Ports> for ReserveStockCommand {
        type Output = ();

        async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
            ports.log.lock().unwrap().push("stock reserved");

            let stock_reserved = StockReserved;

            Ok(Executed {
                output: (),
                events: vec![Box::new(stock_reserved)],
            })
        }
    }

    #[async_trait]
    impl Command<Ports> for ShipOrderCommand {
        type Output = ();

        async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
            ports.log.lock().unwrap().push("order shipped");

            Ok(Executed {
                output: (),
                events: vec![],
            })
        }
    }

    #[async_trait]
    impl Command<Ports> for FailingCommand {
        type Output = ();

        async fn execute(&self, _: &Ports) -> Result<Executed<(), Ports>, Error> {
            Err(DomainError::Violations(vec!["always fails"]))?
        }
    }

    impl DomainEvent<Ports> for OrderPlaced {
        fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
            let has_items = self.has_items;

            Invariants::new(vec![Invariant::new("order has items", move || has_items)])
                .enforce()?;

            Ok(Policies::new(vec![Policy::new(
                "reserve stock",
                || true,
                || Box::new(ReserveStockCommand),
            )])
            .trigger())
        }
    }

    impl DomainEvent<Ports> for StockReserved {
        fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<Ports>>> {
            Ok(Policies::new(vec![Policy::new(
                "ship order",
                || true,
                || Box::new(ShipOrderCommand),
            )])
            .trigger())
        }
    }

    #[tokio::test]
    async fn inline_processor_runs_the_whole_chain_before_returning() {
        let ports = Arc::new(Ports::default());
        let sync_policy_processor = InlinePolicyProcessor::new(Arc::clone(&ports));

        sync_policy_processor
            .send_command(Box::new(PlaceOrderCommand))
            .await
            .unwrap();

        assert_eq!(
            *ports.log.lock().unwrap(),
            vec!["order placed", "stock reserved", "order shipped"]
        );
    }

    #[tokio::test]
    async fn inline_processor_returns_the_error_of_a_failing_command() {
        let ports = Arc::new(Ports::default());
        let sync_policy_processor = InlinePolicyProcessor::new(Arc::clone(&ports));

        let result = sync_policy_processor
            .send_command(Box::new(FailingCommand))
            .await;

        assert!(matches!(result, Err(Error::Domain(_))));
    }

    #[tokio::test]
    async fn send_events_sends_the_commands_of_the_fired_policies() {
        let ports = Arc::new(Ports::default());
        let sync_policy_processor = InlinePolicyProcessor::new(Arc::clone(&ports));

        let place_order_events: Vec<Box<dyn DomainEvent<Ports>>> =
            vec![Box::new(OrderPlaced { has_items: true })];
        let fired_policies = sync_policy_processor
            .send_events(place_order_events)
            .await
            .unwrap();

        assert_eq!(fired_policies, vec!["reserve stock"]);
        assert_eq!(
            *ports.log.lock().unwrap(),
            vec!["stock reserved", "order shipped"]
        );
    }

    #[tokio::test]
    async fn send_events_sends_nothing_when_an_event_is_invalid() {
        let ports = Arc::new(Ports::default());
        let sync_policy_processor = InlinePolicyProcessor::new(Arc::clone(&ports));

        let events: Vec<Box<dyn DomainEvent<Ports>>> = vec![
            Box::new(StockReserved),
            Box::new(OrderPlaced { has_items: false }),
        ];
        let result = sync_policy_processor.send_events(events).await;

        assert!(matches!(result, Err(Error::Domain(_))));
        assert!(ports.log.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn tokio_processor_runs_the_whole_chain_before_shutdown_returns() {
        let ports = Arc::new(Ports::default());
        let async_policy_processor = TokioPolicyProcessor::spawn(Arc::clone(&ports), |_| {});

        async_policy_processor
            .send_command(Box::new(PlaceOrderCommand))
            .await
            .unwrap();
        async_policy_processor.shutdown().await.unwrap();

        assert_eq!(
            *ports.log.lock().unwrap(),
            vec!["order placed", "stock reserved", "order shipped"]
        );
    }

    #[tokio::test]
    async fn tokio_processor_sends_a_failing_command_to_on_error_and_moves_on() {
        let ports = Arc::new(Ports::default());
        let errors = Arc::new(Mutex::new(vec![]));
        let captured = Arc::clone(&errors);
        let async_policy_processor =
            TokioPolicyProcessor::spawn(Arc::clone(&ports), move |error| {
                captured.lock().unwrap().push(error.to_string())
            });

        async_policy_processor
            .send_command(Box::new(FailingCommand))
            .await
            .unwrap();
        async_policy_processor
            .send_command(Box::new(ShipOrderCommand))
            .await
            .unwrap();
        async_policy_processor.shutdown().await.unwrap();

        assert_eq!(
            *errors.lock().unwrap(),
            vec![r#"violated: ["always fails"]"#]
        );
        assert_eq!(*ports.log.lock().unwrap(), vec!["order shipped"]);
    }
}
