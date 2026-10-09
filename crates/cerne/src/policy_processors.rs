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
pub trait PolicyProcessor<CompositionRoot: 'static>: Send + Sync {
    /// Runs the command, and then the commands its events trigger in turn.
    async fn send_command(&self, command: Box<dyn Command<CompositionRoot, Output = ()>>) -> Result<(), Error>;

    /// Triggers the policies of every event and sends the commands they return; returns the names of the policies that fired.
    ///
    /// If the invariants of any event fail, no command is sent.
    async fn send_events(
        &self,
        events: Vec<Box<dyn DomainEvent<CompositionRoot>>>,
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
/// # use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, policy};
/// # use cerne::{Error, async_trait};
/// # struct CompositionRoot;
/// # #[derive(serde::Serialize)]
/// # struct ReserveStockCommand;
/// # #[async_trait]
/// # impl Command<CompositionRoot> for ReserveStockCommand {
/// #     type Output = ();
/// #
/// #     async fn execute(&self, _: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> { Ok(Executed { output: (), events: vec![] }) }
/// # }
/// # struct OrderPlaced;
/// # impl DomainEvent<CompositionRoot> for OrderPlaced {
/// #     fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> {
/// #         Ok(Policies::trigger([policy!("reserve stock", true, ReserveStockCommand)]))
/// #     }
/// # }
/// use cerne::application::{InlinePolicyProcessor, PolicyProcessor};
/// use std::sync::Arc;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
/// let sync_policy_processor = InlinePolicyProcessor::new(Arc::new(CompositionRoot));
///
/// let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> = vec![Box::new(OrderPlaced)];
/// let fired_policies = sync_policy_processor.send_events(place_order_events).await?; // ReserveStockCommand already ran
///
/// assert_eq!(fired_policies, vec!["reserve stock"]);
/// # Ok(())
/// # }
/// ```
pub struct InlinePolicyProcessor<CompositionRoot> {
    composition_root: Arc<CompositionRoot>,
}

impl<CompositionRoot> InlinePolicyProcessor<CompositionRoot> {
    pub fn new(composition_root: Arc<CompositionRoot>) -> Self {
        Self { composition_root }
    }
}

#[async_trait]
impl<CompositionRoot: Send + Sync + 'static> PolicyProcessor<CompositionRoot>
    for InlinePolicyProcessor<CompositionRoot>
{
    async fn send_command(&self, command: Box<dyn Command<CompositionRoot, Output = ()>>) -> Result<(), Error> {
        let mut pending = VecDeque::from([command]);

        while let Some(command) = pending.pop_front() {
            pending.extend(run(command, &self.composition_root).await?);
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
/// # use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, policy};
/// # use cerne::{Error, async_trait};
/// # struct CompositionRoot;
/// # #[derive(serde::Serialize)]
/// # struct ReserveStockCommand;
/// # #[async_trait]
/// # impl Command<CompositionRoot> for ReserveStockCommand {
/// #     type Output = ();
/// #
/// #     async fn execute(&self, _: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> { Ok(Executed { output: (), events: vec![] }) }
/// # }
/// # struct OrderPlaced;
/// # impl DomainEvent<CompositionRoot> for OrderPlaced {
/// #     fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> {
/// #         Ok(Policies::trigger([policy!("reserve stock", true, ReserveStockCommand)]))
/// #     }
/// # }
/// use cerne::application::{PolicyProcessor, TokioPolicyProcessor};
/// use std::sync::Arc;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Error> {
/// let async_policy_processor = TokioPolicyProcessor::spawn(Arc::new(CompositionRoot), |error| eprintln!("{error}"));
///
/// let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> = vec![Box::new(OrderPlaced)];
/// async_policy_processor.send_events(place_order_events).await?; // ReserveStockCommand runs in the background
///
/// async_policy_processor.shutdown().await?; // waits for every command sent
/// # Ok(())
/// # }
/// ```
pub struct TokioPolicyProcessor<CompositionRoot> {
    commands: mpsc::UnboundedSender<Box<dyn Command<CompositionRoot, Output = ()>>>,
    task: JoinHandle<()>,
}

impl<CompositionRoot: Send + Sync + 'static> TokioPolicyProcessor<CompositionRoot> {
    /// Spawns the task. A failing command goes to `on_error`, and the task moves on to the next one.
    /// Must be called inside a `tokio` runtime.
    pub fn spawn(composition_root: Arc<CompositionRoot>, on_error: impl Fn(Error) + Send + 'static) -> Self {
        let (commands, mut received) = mpsc::unbounded_channel::<Box<dyn Command<CompositionRoot, Output = ()>>>();

        let task = tokio::spawn(async move {
            while let Some(command) = received.recv().await {
                let mut pending = VecDeque::from([command]);

                while let Some(command) = pending.pop_front() {
                    match run(command, &composition_root).await {
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
impl<CompositionRoot: Send + Sync + 'static> PolicyProcessor<CompositionRoot>
    for TokioPolicyProcessor<CompositionRoot>
{
    async fn send_command(&self, command: Box<dyn Command<CompositionRoot, Output = ()>>) -> Result<(), Error> {
        self.commands
            .send(command)
            .map_err(|_| InfrastructureError::from(anyhow::anyhow!("policy processor stopped")))?;

        Ok(())
    }
}

/// Runs one command and returns the commands its events trigger.
async fn run<CompositionRoot>(
    command: Box<dyn Command<CompositionRoot, Output = ()>>,
    composition_root: &CompositionRoot,
) -> Result<Vec<Box<dyn Command<CompositionRoot, Output = ()>>>, Error> {
    let mut next = vec![];
    let execution = command.execute(composition_root).await?;

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
    use crate::invariant;
    use crate::invariants::Invariants;
    use crate::policies::{FiredPolicy, Policies};
    use crate::policy;
    use std::sync::Mutex;

    #[derive(Default)]
    struct CompositionRoot {
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
    impl Command<CompositionRoot> for PlaceOrderCommand {
        type Output = ();

        async fn execute(&self, composition_root: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
            composition_root.log.lock().unwrap().push("order placed");

            let order_placed = OrderPlaced { has_items: true };

            Ok(Executed {
                output: (),
                events: vec![Box::new(order_placed)],
            })
        }
    }

    #[async_trait]
    impl Command<CompositionRoot> for ReserveStockCommand {
        type Output = ();

        async fn execute(&self, composition_root: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
            composition_root.log.lock().unwrap().push("stock reserved");

            let stock_reserved = StockReserved;

            Ok(Executed {
                output: (),
                events: vec![Box::new(stock_reserved)],
            })
        }
    }

    #[async_trait]
    impl Command<CompositionRoot> for ShipOrderCommand {
        type Output = ();

        async fn execute(&self, composition_root: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
            composition_root.log.lock().unwrap().push("order shipped");

            Ok(Executed {
                output: (),
                events: vec![],
            })
        }
    }

    #[async_trait]
    impl Command<CompositionRoot> for FailingCommand {
        type Output = ();

        async fn execute(&self, _: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
            Err(DomainError::Violations(vec!["always fails"]))?
        }
    }

    impl DomainEvent<CompositionRoot> for OrderPlaced {
        fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> {
            let has_items = self.has_items;

            Invariants::enforce([invariant!("order has items", has_items)])?;

            let reserve_stock_policy = policy!("reserve stock", true, ReserveStockCommand);

            Ok(Policies::trigger([reserve_stock_policy]))
        }
    }

    impl DomainEvent<CompositionRoot> for StockReserved {
        fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> {
            let ship_order_policy = policy!("ship order", true, ShipOrderCommand);

            Ok(Policies::trigger([ship_order_policy]))
        }
    }

    #[tokio::test]
    async fn inline_processor_runs_the_whole_chain_before_returning() {
        let composition_root = Arc::new(CompositionRoot::default());
        let sync_policy_processor = InlinePolicyProcessor::new(Arc::clone(&composition_root));

        sync_policy_processor
            .send_command(Box::new(PlaceOrderCommand))
            .await
            .unwrap();

        assert_eq!(*composition_root.log.lock().unwrap(), vec!["order placed", "stock reserved", "order shipped"]);
    }

    #[tokio::test]
    async fn inline_processor_returns_the_error_of_a_failing_command() {
        let composition_root = Arc::new(CompositionRoot::default());
        let sync_policy_processor = InlinePolicyProcessor::new(Arc::clone(&composition_root));

        let result = sync_policy_processor
            .send_command(Box::new(FailingCommand))
            .await;

        assert!(matches!(result, Err(Error::Domain(_))));
    }

    #[tokio::test]
    async fn send_events_sends_the_commands_of_the_fired_policies() {
        let composition_root = Arc::new(CompositionRoot::default());
        let sync_policy_processor = InlinePolicyProcessor::new(Arc::clone(&composition_root));

        let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> =
            vec![Box::new(OrderPlaced { has_items: true })];
        let fired_policies = sync_policy_processor
            .send_events(place_order_events)
            .await
            .unwrap();

        assert_eq!(fired_policies, vec!["reserve stock"]);
        assert_eq!(*composition_root.log.lock().unwrap(), vec!["stock reserved", "order shipped"]);
    }

    #[tokio::test]
    async fn send_events_sends_nothing_when_an_event_is_invalid() {
        let composition_root = Arc::new(CompositionRoot::default());
        let sync_policy_processor = InlinePolicyProcessor::new(Arc::clone(&composition_root));

        let events: Vec<Box<dyn DomainEvent<CompositionRoot>>> =
            vec![Box::new(StockReserved), Box::new(OrderPlaced { has_items: false })];
        let result = sync_policy_processor.send_events(events).await;

        assert!(matches!(result, Err(Error::Domain(_))));
        assert!(composition_root.log.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn tokio_processor_runs_the_whole_chain_before_shutdown_returns() {
        let composition_root = Arc::new(CompositionRoot::default());
        let async_policy_processor = TokioPolicyProcessor::spawn(Arc::clone(&composition_root), |_| {});

        async_policy_processor
            .send_command(Box::new(PlaceOrderCommand))
            .await
            .unwrap();
        async_policy_processor.shutdown().await.unwrap();

        assert_eq!(*composition_root.log.lock().unwrap(), vec!["order placed", "stock reserved", "order shipped"]);
    }

    #[tokio::test]
    async fn tokio_processor_sends_a_failing_command_to_on_error_and_moves_on() {
        let composition_root = Arc::new(CompositionRoot::default());
        let errors = Arc::new(Mutex::new(vec![]));
        let captured = Arc::clone(&errors);
        let async_policy_processor = TokioPolicyProcessor::spawn(Arc::clone(&composition_root), move |error| {
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

        assert_eq!(*errors.lock().unwrap(), vec![r#"violated: ["always fails"]"#]);
        assert_eq!(*composition_root.log.lock().unwrap(), vec!["order shipped"]);
    }
}
