use crate::domain_events::DomainEvent;
use crate::errors::Error;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

type OnError = Arc<dyn Fn(Error) + Send + Sync>;

/// The event bus that runs one chain at a time (FIFO): for each event, it triggers its policies, executes the command
/// of each one, and runs the events that command returns, until the chain ends. `publish` returns then, and the next
/// `publish` only starts then, in the order the calls arrived.
///
/// The bus opens no transaction: each command opens its own. A failing command, or an event whose invariants fail,
/// goes to `on_error`, and the bus moves on to the next one. Nothing runs again: how each policy survives a failure
/// (or the process dying) is up to the application.
///
/// A command returns its events in [`Executed`](crate::application::Executed) and never publishes them: the bus queues
/// them in the chain it is already running. A command that called `publish` on this bus would wait for its own chain
/// to end, forever.
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
/// use cerne::application::SyncEventBus;
/// use std::sync::Arc;
///
/// # #[tokio::main]
/// # async fn main() {
/// let composition_root = Arc::new(CompositionRoot);
///
/// let sync_event_bus = SyncEventBus::new(composition_root, |error| eprintln!("{error}"));
///
/// let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> = vec![Box::new(OrderPlaced)];
///
/// sync_event_bus.publish(place_order_events).await; // the whole chain already ran
/// # }
/// ```
pub struct SyncEventBus<CompositionRoot> {
    composition_root: Arc<CompositionRoot>,
    on_error: OnError,
    one_chain_at_a_time: Mutex<()>,
}

impl<CompositionRoot> SyncEventBus<CompositionRoot> {
    pub fn new(composition_root: Arc<CompositionRoot>, on_error: impl Fn(Error) + Send + Sync + 'static) -> Self {
        Self {
            composition_root,
            on_error: Arc::new(on_error),
            one_chain_at_a_time: Mutex::new(()),
        }
    }

    /// Runs the chain of these events and returns when it ends. A `publish` that arrives meanwhile waits its turn.
    pub async fn publish(&self, events: Vec<Box<dyn DomainEvent<CompositionRoot>>>) {
        // Tokio's mutex is fair: the chains run in the order the calls arrived.
        let _chain_running = self.one_chain_at_a_time.lock().await;

        let mut pending_events = VecDeque::from(events);

        while let Some(event) = pending_events.pop_front() {
            pending_events.extend(run_policies(event, &self.composition_root, &*self.on_error).await);
        }
    }
}

/// The event bus that runs every event at once: each event starts in its own Tokio task, without waiting for the
/// others, and the events its commands return are published back to the bus, where they start at once too. On the
/// multi-thread runtime (the default of `#[tokio::main]`), the tasks run in parallel on every core.
///
/// Like the [`SyncEventBus`], it opens no transaction, and a failure goes to `on_error`. `publish` returns as soon as
/// the events have started: nothing tells when a chain ends. A command that does heavy CPU work (hashing, checking
/// signatures) holds a worker thread until it ends: it should move that work to `tokio::task::spawn_blocking`.
///
/// ```
/// # use cerne::domain::{DomainEvent, EnforcementResult, FiredPolicy};
/// # struct CompositionRoot;
/// # struct OrderPlaced;
/// # impl DomainEvent<CompositionRoot> for OrderPlaced {
/// #     fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> { Ok(vec![]) }
/// # }
/// use cerne::application::AsyncEventBus;
/// use std::sync::Arc;
///
/// # #[tokio::main]
/// # async fn main() {
/// let composition_root = Arc::new(CompositionRoot);
///
/// let async_event_bus = AsyncEventBus::new(composition_root, |error| eprintln!("{error}"));
///
/// let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> = vec![Box::new(OrderPlaced)];
///
/// async_event_bus.publish(place_order_events); // the chain runs in the background
/// # }
/// ```
pub struct AsyncEventBus<CompositionRoot> {
    composition_root: Arc<CompositionRoot>,
    on_error: OnError,
}

impl<CompositionRoot> AsyncEventBus<CompositionRoot> {
    pub fn new(composition_root: Arc<CompositionRoot>, on_error: impl Fn(Error) + Send + Sync + 'static) -> Self {
        Self {
            composition_root,
            on_error: Arc::new(on_error),
        }
    }
}

// By hand: `#[derive(Clone)]` would ask for `CompositionRoot: Clone`, and only the `Arc`s are cloned.
impl<CompositionRoot> Clone for AsyncEventBus<CompositionRoot> {
    fn clone(&self) -> Self {
        Self {
            composition_root: Arc::clone(&self.composition_root),
            on_error: Arc::clone(&self.on_error),
        }
    }
}

impl<CompositionRoot: Send + Sync + 'static> AsyncEventBus<CompositionRoot> {
    /// Starts each event in its own task and returns at once. Must be called inside a Tokio runtime.
    pub fn publish(&self, events: Vec<Box<dyn DomainEvent<CompositionRoot>>>) {
        for event in events {
            let async_event_bus = self.clone();

            tokio::spawn(async move {
                let composition_root = &async_event_bus.composition_root;
                let next_events = run_policies(event, composition_root, &*async_event_bus.on_error).await;

                async_event_bus.publish(next_events);
            });
        }
    }
}

/// Triggers the policies of one event and executes the command of each; returns the events those commands produced.
async fn run_policies<CompositionRoot>(
    event: Box<dyn DomainEvent<CompositionRoot>>,
    composition_root: &CompositionRoot,
    on_error: &(dyn Fn(Error) + Send + Sync),
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
            // Gives way to the other tasks, like a real adapter waiting on its database.
            tokio::task::yield_now().await;

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

    fn errors_and_on_error() -> (Arc<Mutex<Vec<String>>>, impl Fn(Error) + Send + Sync + 'static) {
        let errors = Arc::new(Mutex::new(vec![]));
        let captured_errors = Arc::clone(&errors);

        (errors, move |error: Error| captured_errors.lock().unwrap().push(error.to_string()))
    }

    #[tokio::test]
    async fn sync_event_bus_runs_the_whole_chain_before_publish_returns() {
        let composition_root = Arc::new(CompositionRoot::default());
        let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), |_| {});

        let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> =
            vec![Box::new(OrderPlaced { has_items: true })];

        sync_event_bus.publish(place_order_events).await;

        assert_eq!(*composition_root.log.lock().unwrap(), vec!["stock reserved", "order shipped"]);
    }

    #[tokio::test]
    async fn sync_event_bus_sends_a_failing_command_to_on_error_and_moves_on() {
        let composition_root = Arc::new(CompositionRoot::default());
        let (errors, on_error) = errors_and_on_error();
        let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), on_error);

        let events: Vec<Box<dyn DomainEvent<CompositionRoot>>> =
            vec![Box::new(PaymentRefused), Box::new(StockReserved)];

        sync_event_bus.publish(events).await;

        assert_eq!(*errors.lock().unwrap(), vec![r#"violated: ["always fails"]"#]);
        assert_eq!(*composition_root.log.lock().unwrap(), vec!["order shipped"]);
    }

    #[tokio::test]
    async fn sync_event_bus_runs_nothing_for_an_invalid_event() {
        let composition_root = Arc::new(CompositionRoot::default());
        let (errors, on_error) = errors_and_on_error();
        let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), on_error);

        let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> =
            vec![Box::new(OrderPlaced { has_items: false })];

        sync_event_bus.publish(place_order_events).await;

        assert_eq!(*errors.lock().unwrap(), vec![r#"violated: ["order has items"]"#]);
        assert!(composition_root.log.lock().unwrap().is_empty());
    }

    // --- Two chains at once: the second waits for the first -------------------------------------------------------

    struct CustomerRegistered;
    struct WelcomeCustomerCommand;

    #[async_trait]
    impl Command<CompositionRoot> for WelcomeCustomerCommand {
        type Output = ();

        async fn execute(&self, composition_root: &CompositionRoot) -> Result<Executed<(), CompositionRoot>, Error> {
            composition_root
                .log
                .lock()
                .unwrap()
                .push("customer welcomed");

            Ok(Executed {
                output: (),
                events: vec![],
            })
        }
    }

    impl DomainEvent<CompositionRoot> for CustomerRegistered {
        fn trigger_policies(&self) -> EnforcementResult<Vec<FiredPolicy<CompositionRoot>>> {
            let welcome_customer_policy = policy!("welcome customer", true, WelcomeCustomerCommand);

            Ok(Policies::trigger([welcome_customer_policy]))
        }
    }

    #[tokio::test]
    async fn sync_event_bus_runs_one_chain_at_a_time_in_the_order_they_arrived() {
        let composition_root = Arc::new(CompositionRoot::default());
        let sync_event_bus = SyncEventBus::new(Arc::clone(&composition_root), |_| {});

        let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> =
            vec![Box::new(OrderPlaced { has_items: true })];
        let register_customer_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> = vec![Box::new(CustomerRegistered)];

        // "reserve stock" gives way in the middle of the first chain: the second one must wait anyway.
        tokio::join!(sync_event_bus.publish(place_order_events), sync_event_bus.publish(register_customer_events));

        assert_eq!(*composition_root.log.lock().unwrap(), vec!["stock reserved", "order shipped", "customer welcomed"]);
    }

    // An axum handler or a `tokio::spawn` only takes a `publish` whose future is `Send`.
    fn _sync_event_bus_publish_is_send(sync_event_bus: &'static SyncEventBus<CompositionRoot>) -> impl Send {
        sync_event_bus.publish(vec![])
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

    #[tokio::test(flavor = "multi_thread")]
    async fn async_event_bus_runs_two_events_at_once() {
        let (finished, mut received) = mpsc::unbounded_channel();
        let composition_root = Arc::new(GateCompositionRoot {
            gate: Notify::new(),
            finished,
        });
        let async_event_bus = AsyncEventBus::new(composition_root, |_| {});

        // One at a time, the first would wait forever for the gate the second opens.
        let events: Vec<Box<dyn DomainEvent<GateCompositionRoot>>> = vec![Box::new(GateClosed), Box::new(GateOpened)];

        async_event_bus.publish(events);

        let both_finished = async { [received.recv().await.unwrap(), received.recv().await.unwrap()] };
        let finished_in_order = tokio::time::timeout(Duration::from_secs(5), both_finished)
            .await
            .unwrap();

        assert_eq!(finished_in_order, ["opened", "waited"]);
    }

    #[tokio::test]
    async fn async_event_bus_publishes_the_events_of_a_command_back_to_itself() {
        let composition_root = Arc::new(CompositionRoot::default());
        let async_event_bus = AsyncEventBus::new(Arc::clone(&composition_root), |_| {});

        let place_order_events: Vec<Box<dyn DomainEvent<CompositionRoot>>> =
            vec![Box::new(OrderPlaced { has_items: true })];

        async_event_bus.publish(place_order_events);

        for _ in 0..100 {
            if composition_root.log.lock().unwrap().len() == 2 {
                break;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert_eq!(*composition_root.log.lock().unwrap(), vec!["stock reserved", "order shipped"]);
    }
}
