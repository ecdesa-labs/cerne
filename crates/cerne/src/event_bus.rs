use crate::domain_events::DomainEvent;
use crate::errors::Error;
use actix::{Actor, AsyncContext, AtomicResponse, Context, Handler, Message, WrapFuture};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

/// The events a command returned, published by whoever called it: `sync_event_bus.send(PublishEvents(events))`.
pub struct PublishEvents<CompositionRoot>(pub Vec<Box<dyn DomainEvent<CompositionRoot>>>);

impl<CompositionRoot: 'static> Message for PublishEvents<CompositionRoot> {
    type Result = ();
}

/// The event bus that runs one chain at a time (FIFO): an Actix actor that, for each event, triggers its policies,
/// executes the command of each one, and publishes the events that command returns, until the chain ends. The next
/// `PublishEvents` only starts then.
///
/// The bus opens no transaction: each command opens its own. A failing command, or an event whose invariants fail,
/// goes to `on_error`, and the bus moves on to the next one. Nothing runs again: how each policy survives a failure
/// (or the process dying) is up to the application.
///
/// ```
/// # use cerne::application::{Command, Executed};
/// # use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy, Policies, policy};
/// # use cerne::{Error, async_trait};
/// # struct CompositionRoot;
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
/// use actix::Actor;
/// use cerne::application::{PublishEvents, SyncEventBus};
/// use std::sync::Arc;
///
/// # #[actix::main]
/// # async fn main() {
/// let composition_root = Arc::new(CompositionRoot);
///
/// let sync_event_bus = SyncEventBus::new(composition_root, |error| eprintln!("{error}")).start();
///
/// let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> = vec![Box::new(OrderPlaced)];
///
/// sync_event_bus.send(PublishEvents(place_order_events)).await.unwrap(); // the whole chain already ran
/// # }
/// ```
pub struct SyncEventBus<CompositionRoot> {
    composition_root: Arc<CompositionRoot>,
    on_error: Rc<dyn Fn(Error)>,
}

impl<CompositionRoot> SyncEventBus<CompositionRoot> {
    pub fn new(composition_root: Arc<CompositionRoot>, on_error: impl Fn(Error) + 'static) -> Self {
        Self {
            composition_root,
            on_error: Rc::new(on_error),
        }
    }
}

impl<CompositionRoot: Send + Sync + 'static> Actor for SyncEventBus<CompositionRoot> {
    type Context = Context<Self>;
}

impl<CompositionRoot: Send + Sync + 'static> Handler<PublishEvents<CompositionRoot>> for SyncEventBus<CompositionRoot> {
    // The actor reads no other message while the chain runs.
    type Result = AtomicResponse<Self, ()>;

    fn handle(&mut self, publish_events: PublishEvents<CompositionRoot>, _: &mut Context<Self>) -> Self::Result {
        let composition_root = Arc::clone(&self.composition_root);
        let on_error = Rc::clone(&self.on_error);

        let chain = async move {
            let mut pending_events = VecDeque::from(publish_events.0);

            while let Some(event) = pending_events.pop_front() {
                pending_events.extend(run_policies(event, &composition_root, &*on_error).await);
            }
        };

        AtomicResponse::new(Box::pin(chain.into_actor(self)))
    }
}

/// The event bus that runs every event at once: each event starts as soon as it arrives, without waiting for the
/// others, and the events its commands return are published back to the bus, where they start at once too.
///
/// Like the [`SyncEventBus`], it opens no transaction, and a failure goes to `on_error`. `send` returns as soon as the
/// events have started: nothing tells when a chain ends.
///
/// ```
/// # use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy};
/// # struct CompositionRoot;
/// # struct OrderPlaced;
/// # impl DomainEvent<CompositionRoot> for OrderPlaced {
/// #     fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> { Ok(vec![]) }
/// # }
/// use actix::Actor;
/// use cerne::application::{AsyncEventBus, PublishEvents};
/// use std::sync::Arc;
///
/// # #[actix::main]
/// # async fn main() {
/// let composition_root = Arc::new(CompositionRoot);
///
/// let async_event_bus = AsyncEventBus::new(composition_root, |error| eprintln!("{error}")).start();
///
/// let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> = vec![Box::new(OrderPlaced)];
///
/// async_event_bus.do_send(PublishEvents(place_order_events)); // the chain runs in the background
/// # }
/// ```
pub struct AsyncEventBus<CompositionRoot> {
    composition_root: Arc<CompositionRoot>,
    on_error: Rc<dyn Fn(Error)>,
}

impl<CompositionRoot> AsyncEventBus<CompositionRoot> {
    pub fn new(composition_root: Arc<CompositionRoot>, on_error: impl Fn(Error) + 'static) -> Self {
        Self {
            composition_root,
            on_error: Rc::new(on_error),
        }
    }
}

impl<CompositionRoot: Send + Sync + 'static> Actor for AsyncEventBus<CompositionRoot> {
    type Context = Context<Self>;
}

impl<CompositionRoot: Send + Sync + 'static> Handler<PublishEvents<CompositionRoot>>
    for AsyncEventBus<CompositionRoot>
{
    type Result = ();

    fn handle(&mut self, publish_events: PublishEvents<CompositionRoot>, context: &mut Context<Self>) {
        for event in publish_events.0 {
            let composition_root = Arc::clone(&self.composition_root);
            let on_error = Rc::clone(&self.on_error);
            let async_event_bus = context.address();

            let chain = async move {
                let next_events = run_policies(event, &composition_root, &*on_error).await;

                if !next_events.is_empty() {
                    async_event_bus.do_send(PublishEvents(next_events));
                }
            };

            context.spawn(chain.into_actor(self));
        }
    }
}

/// Triggers the policies of one event and executes the command of each; returns the events those commands produced.
async fn run_policies<CompositionRoot>(
    event: Box<dyn DomainEvent<CompositionRoot>>,
    composition_root: &CompositionRoot,
    on_error: &dyn Fn(Error),
) -> Vec<Box<dyn DomainEvent<CompositionRoot>>> {
    let fired_policies = match event.trigger_policies() {
        Ok(fired_policies) => fired_policies,
        Err(error) => {
            on_error(error.into());

            return vec![];
        }
    };

    let mut next_events = vec![];

    for fired_policy in fired_policies {
        match fired_policy.command.execute(composition_root).await {
            Ok(execution) => next_events.extend(execution.events),
            Err(error) => on_error(error),
        }
    }

    next_events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Command, Executed};
    use crate::errors::{DomainError, EnforcementResult};
    use crate::invariant;
    use crate::invariants::Invariants;
    use crate::policies::{FiredPolicy, Policies};
    use crate::policy;
    use async_trait::async_trait;
    use std::sync::Mutex;
    use std::time::Duration;
    use tokio::sync::{Notify, mpsc};

    // --- Place order → OrderPlaced → "reserve stock" → StockReserved → "ship order" ---------------------------------

    #[derive(Default)]
    struct CompositionRoot {
        log: Mutex<Vec<&'static str>>,
    }

    struct ReserveStockCommand;
    struct ShipOrderCommand;
    struct FailingCommand;

    struct OrderPlaced {
        has_items: bool,
    }
    struct StockReserved;
    struct PaymentRefused;

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

    impl DomainEvent<CompositionRoot> for PaymentRefused {
        fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> {
            let cancel_order_policy = policy!("cancel order", true, FailingCommand);

            Ok(Policies::trigger([cancel_order_policy]))
        }
    }

    fn errors_and_on_error() -> (Arc<Mutex<Vec<String>>>, impl Fn(Error) + 'static) {
        let errors = Arc::new(Mutex::new(vec![]));
        let captured_errors = Arc::clone(&errors);

        (errors, move |error: Error| captured_errors.lock().unwrap().push(error.to_string()))
    }

    #[actix::test]
    async fn sync_event_bus_runs_the_whole_chain_before_send_returns() {
        let composition_root = Arc::new(CompositionRoot::default());
        let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), |_| {}).start();

        let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> =
            vec![Box::new(OrderPlaced { has_items: true })];

        sync_event_bus
            .send(PublishEvents(place_order_events))
            .await
            .unwrap();

        assert_eq!(*composition_root.log.lock().unwrap(), vec!["stock reserved", "order shipped"]);
    }

    #[actix::test]
    async fn sync_event_bus_sends_a_failing_command_to_on_error_and_moves_on() {
        let composition_root = Arc::new(CompositionRoot::default());
        let (errors, on_error) = errors_and_on_error();
        let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), on_error).start();

        let events: Vec<Box<dyn DomainEvent<CompositionRoot>>> =
            vec![Box::new(PaymentRefused), Box::new(StockReserved)];

        sync_event_bus.send(PublishEvents(events)).await.unwrap();

        assert_eq!(*errors.lock().unwrap(), vec![r#"violated: ["always fails"]"#]);
        assert_eq!(*composition_root.log.lock().unwrap(), vec!["order shipped"]);
    }

    #[actix::test]
    async fn sync_event_bus_runs_nothing_for_an_invalid_event() {
        let composition_root = Arc::new(CompositionRoot::default());
        let (errors, on_error) = errors_and_on_error();
        let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), on_error).start();

        let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> =
            vec![Box::new(OrderPlaced { has_items: false })];

        sync_event_bus
            .send(PublishEvents(place_order_events))
            .await
            .unwrap();

        assert_eq!(*errors.lock().unwrap(), vec![r#"violated: ["order has items"]"#]);
        assert!(composition_root.log.lock().unwrap().is_empty());
    }

    // --- Two events at once: the first waits for the second ----------------------------------------------------------

    struct GateCompositionRoot {
        gate: Notify,
        finished: mpsc::UnboundedSender<&'static str>,
    }

    struct WaitForTheGateCommand;
    struct OpenTheGateCommand;

    struct GateClosed;
    struct GateOpened;

    #[async_trait]
    impl Command<GateCompositionRoot> for WaitForTheGateCommand {
        type Output = ();

        async fn execute(
            &self,
            composition_root: &GateCompositionRoot,
        ) -> Result<Executed<(), GateCompositionRoot>, Error> {
            composition_root.gate.notified().await;
            composition_root.finished.send("waited").unwrap();

            Ok(Executed {
                output: (),
                events: vec![],
            })
        }
    }

    #[async_trait]
    impl Command<GateCompositionRoot> for OpenTheGateCommand {
        type Output = ();

        async fn execute(
            &self,
            composition_root: &GateCompositionRoot,
        ) -> Result<Executed<(), GateCompositionRoot>, Error> {
            composition_root.gate.notify_one();
            composition_root.finished.send("opened").unwrap();

            Ok(Executed {
                output: (),
                events: vec![],
            })
        }
    }

    impl DomainEvent<GateCompositionRoot> for GateClosed {
        fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<GateCompositionRoot>>> {
            let wait_for_the_gate_policy = policy!("wait for the gate", true, WaitForTheGateCommand);

            Ok(Policies::trigger([wait_for_the_gate_policy]))
        }
    }

    impl DomainEvent<GateCompositionRoot> for GateOpened {
        fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<GateCompositionRoot>>> {
            let open_the_gate_policy = policy!("open the gate", true, OpenTheGateCommand);

            Ok(Policies::trigger([open_the_gate_policy]))
        }
    }

    #[actix::test]
    async fn async_event_bus_runs_two_events_at_once() {
        let (finished, mut received) = mpsc::unbounded_channel();
        let composition_root = Arc::new(GateCompositionRoot {
            gate: Notify::new(),
            finished,
        });
        let async_event_bus = AsyncEventBus::new(composition_root, |_| {}).start();

        // One at a time, the first would wait forever for the gate the second opens.
        let events: Vec<Box<dyn DomainEvent<GateCompositionRoot>>> = vec![Box::new(GateClosed), Box::new(GateOpened)];

        async_event_bus.do_send(PublishEvents(events));

        let both_finished = async { [received.recv().await.unwrap(), received.recv().await.unwrap()] };
        let finished_in_order = tokio::time::timeout(Duration::from_secs(5), both_finished)
            .await
            .unwrap();

        assert_eq!(finished_in_order, ["opened", "waited"]);
    }

    #[actix::test]
    async fn async_event_bus_publishes_the_events_of_a_command_back_to_itself() {
        let composition_root = Arc::new(CompositionRoot::default());
        let async_event_bus = AsyncEventBus::new(Arc::clone(&composition_root), |_| {}).start();

        let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> =
            vec![Box::new(OrderPlaced { has_items: true })];

        async_event_bus.do_send(PublishEvents(place_order_events));

        for _ in 0..100 {
            if composition_root.log.lock().unwrap().len() == 2 {
                break;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert_eq!(*composition_root.log.lock().unwrap(), vec!["stock reserved", "order shipped"]);
    }
}
