use crate::commands::Command;
use crate::domain_events::DomainEvent;
use crate::errors::{Error, InfrastructureError};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

/// A command a policy fired, as the outbox stores it: its name and its fields in JSON.
#[derive(Debug, Clone, PartialEq)]
pub struct OutboxEntry {
    /// `ChargeOrderCommand` → `"charge_order"`.
    pub command: String,
    pub json: String,
}

/// A command waiting in the outbox, with the id of its row.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredCommand {
    pub id: i64,
    pub command: String,
    pub json: String,
}

/// The port of the outbox table: the commands policies fired, stored in the same transaction as the aggregate that
/// produced the events. A command in the table runs at least once, even if the process dies.
///
/// The table is the same in every database (`cerne_outbox`); the adapters are `cerne::sqlite::SqliteOutbox` and,
/// with the `postgres` feature, `cerne::postgres::PostgresOutbox`. A project without a database uses the
/// [`InMemoryOutbox`].
#[async_trait]
pub trait Outbox<CompositionRoot: 'static>: Send + Sync {
    /// Stores one command, to run later in a transaction of its own.
    async fn store(&self, outbox_entry: OutboxEntry) -> Result<(), Error>;

    /// The oldest command still pending, if any.
    async fn next_pending(&self) -> Result<Option<StoredCommand>, Error>;

    async fn mark_done(&self, id: i64) -> Result<(), Error>;

    /// A failed command keeps its row, with the error, and does not run again by itself.
    async fn mark_failed(&self, id: i64, error: &str) -> Result<(), Error>;

    /// Triggers the policies of every event and stores the commands they return; returns the names of the policies
    /// that fired.
    ///
    /// If the invariants of any event fail, no command is stored.
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
            self.store(policy.outbox_entry()?).await?;
            names.push(policy.name);
        }

        Ok(names)
    }
}

/// A composition root that can open a database transaction: the composition root `begin` returns writes every repository
/// and the outbox in that transaction, and `commit` makes it permanent. Dropping it without `commit` rolls everything
/// back.
///
/// External systems (a blockchain, a notifier) are not part of the transaction: `begin` hands the same adapters to
/// the new composition root. That is why calls to them belong in commands fired by policies, which the outbox runs at
/// least once.
#[async_trait]
pub trait TransactionalCompositionRoot: Sized + Send + Sync + 'static {
    async fn begin(&self) -> Result<Self, Error>;

    async fn commit(self) -> Result<(), Error>;

    fn outbox(&self) -> &dyn Outbox<Self>;

    /// What every request of an actor does: begin, execute the command, store the commands of its policies in the
    /// outbox, commit; returns the output of the command. If anything fails, nothing is written.
    async fn execute_in_transaction<C>(&self, command: C) -> Result<C::Output, Error>
    where
        C: Command<Self> + 'static,
    {
        let transaction = self.begin().await?;

        let execution = command.execute(&transaction).await?;

        transaction.outbox().send_events(execution.events).await?;
        transaction.commit().await?;

        Ok(execution.output)
    }
}

type Decode<CompositionRoot> = fn(&str) -> serde_json::Result<Box<dyn Command<CompositionRoot, Output = ()>>>;

/// Turns a row of the outbox back into the command it stores: every command a policy fires is registered here.
///
/// ```ignore
/// let command_registry = CommandRegistry::new()
///     .register::<ReserveStockCommand>()
///     .register::<ChargeOrderCommand>();
/// ```
pub struct CommandRegistry<CompositionRoot> {
    decoders: HashMap<String, Decode<CompositionRoot>>,
}

impl<CompositionRoot: 'static> CommandRegistry<CompositionRoot> {
    pub fn new() -> Self {
        Self {
            decoders: HashMap::new(),
        }
    }

    pub fn register<C>(mut self) -> Self
    where
        C: Command<CompositionRoot, Output = ()> + DeserializeOwned + 'static,
    {
        fn decode<CompositionRoot, C>(json: &str) -> serde_json::Result<Box<dyn Command<CompositionRoot, Output = ()>>>
        where
            C: Command<CompositionRoot, Output = ()> + DeserializeOwned + 'static,
        {
            Ok(Box::new(serde_json::from_str::<C>(json)?))
        }

        self.decoders
            .insert(command_name::<C>(), decode::<CompositionRoot, C>);

        self
    }

    pub fn decode(
        &self,
        stored_command: &StoredCommand,
    ) -> Result<Box<dyn Command<CompositionRoot, Output = ()>>, Error> {
        let decode = self
            .decoders
            .get(&stored_command.command)
            .ok_or_else(|| infrastructure(format!("{} is not in the command registry", stored_command.command)))?;

        let command = decode(&stored_command.json)
            .map_err(|error| infrastructure(format!("cannot read {}: {error}", stored_command.command)))?;

        Ok(command)
    }
}

impl<CompositionRoot: 'static> Default for CommandRegistry<CompositionRoot> {
    fn default() -> Self {
        Self::new()
    }
}

/// What happened to one command of the outbox: `error` is `None` when it ran.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandRun {
    pub command: String,
    pub error: Option<String>,
}

/// The `PolicyProcessor` of the outbox: runs the commands stored in the outbox table, each in a transaction of its own.
///
/// For each command: begin, execute, store the commands its events fire, mark it done, commit. A failing command
/// rolls back, and its row is marked failed with the error. If the process dies halfway, nothing was committed, and
/// the command runs again: it must be idempotent, like a command that checks "not chained yet".
pub struct OutboxPolicyProcessor<CompositionRoot> {
    composition_root: Arc<CompositionRoot>,
    command_registry: Arc<CommandRegistry<CompositionRoot>>,
}

impl<CompositionRoot: TransactionalCompositionRoot> OutboxPolicyProcessor<CompositionRoot> {
    pub fn new(composition_root: Arc<CompositionRoot>, command_registry: CommandRegistry<CompositionRoot>) -> Self {
        Self {
            composition_root,
            command_registry: Arc::new(command_registry),
        }
    }

    /// Runs every pending command, and the commands they store in turn, until the outbox is empty.
    pub async fn run_pending(&self) -> Result<Vec<CommandRun>, Error> {
        let mut command_runs = vec![];

        while let Some(command_run) = self.run_next().await? {
            command_runs.push(command_run);
        }

        Ok(command_runs)
    }

    /// Runs the pending commands, then waits `interval` and looks again, forever. Spawn it in a task of its own.
    pub async fn run_every(&self, interval: Duration, on_error: impl Fn(Error) + Send + Sync) {
        loop {
            match self.run_pending().await {
                Ok(command_runs) => {
                    for CommandRun { command, error } in command_runs {
                        if let Some(error) = error {
                            on_error(infrastructure(format!("{command} failed: {error}")));
                        }
                    }
                }
                Err(error) => on_error(error),
            }

            tokio::time::sleep(interval).await;
        }
    }

    async fn run_next(&self) -> Result<Option<CommandRun>, Error> {
        let transaction = self.composition_root.begin().await?;

        let Some(stored_command) = transaction.outbox().next_pending().await? else {
            return Ok(None);
        };

        match self.execute(&stored_command, &transaction).await {
            Ok(()) => {
                transaction.outbox().mark_done(stored_command.id).await?;
                transaction.commit().await?;

                Ok(Some(CommandRun {
                    command: stored_command.command,
                    error: None,
                }))
            }
            Err(error) => {
                drop(transaction); // rolls back whatever the command wrote

                let error = error.to_string();
                let failure = self.composition_root.begin().await?;

                failure
                    .outbox()
                    .mark_failed(stored_command.id, &error)
                    .await?;
                failure.commit().await?;

                Ok(Some(CommandRun {
                    command: stored_command.command,
                    error: Some(error),
                }))
            }
        }
    }

    async fn execute(&self, stored_command: &StoredCommand, transaction: &CompositionRoot) -> Result<(), Error> {
        let command = self.command_registry.decode(stored_command)?;

        let execution = command.execute(transaction).await?;

        transaction.outbox().send_events(execution.events).await?;

        Ok(())
    }
}

/// The outbox of a project without a database (`cerne new` without `--db`): the commands policies fire wait in
/// memory, and the `OutboxPolicyProcessor` runs them as it runs a table.
///
/// There is no transaction: `begin` returns the same outbox and `commit` does nothing, so a command that fails keeps
/// whatever it already wrote. If the process dies, the commands still waiting are lost. `cerne g db` swaps it for a
/// database and its outbox table.
///
/// ```
/// use cerne::application::{InMemoryOutbox, Outbox, OutboxEntry};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), cerne::Error> {
/// let in_memory_outbox = InMemoryOutbox::new();
///
/// let charge_order = OutboxEntry {
///     command: "charge_order".into(),
///     json: r#"{"order_id":1}"#.into(),
/// };
///
/// Outbox::<()>::store(&in_memory_outbox, charge_order).await?;
///
/// let pending = Outbox::<()>::next_pending(&in_memory_outbox).await?.unwrap();
///
/// assert_eq!(pending.command, "charge_order");
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default)]
pub struct InMemoryOutbox {
    rows: Arc<Mutex<Vec<InMemoryOutboxRow>>>,
}

struct InMemoryOutboxRow {
    stored_command: StoredCommand,
    is_pending: bool,
    error: Option<String>,
}

impl InMemoryOutbox {
    pub fn new() -> Self {
        Self::default()
    }

    /// The same outbox: in memory there is no transaction to open.
    pub async fn begin(&self) -> Result<Self, Error> {
        Ok(self.clone())
    }

    /// Nothing to make permanent: every write is already in memory.
    pub async fn commit(&self) -> Result<(), Error> {
        Ok(())
    }

    /// The error of every command that failed, in the order they were stored.
    pub fn failures(&self) -> Vec<(StoredCommand, String)> {
        self.rows()
            .iter()
            .filter_map(|row| Some((row.stored_command.clone(), row.error.clone()?)))
            .collect()
    }

    fn rows(&self) -> MutexGuard<'_, Vec<InMemoryOutboxRow>> {
        self.rows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn finish(&self, id: i64, error: Option<String>) {
        if let Some(row) = self
            .rows()
            .iter_mut()
            .find(|row| row.stored_command.id == id)
        {
            row.is_pending = false;
            row.error = error;
        }
    }
}

#[async_trait]
impl<CompositionRoot: 'static> Outbox<CompositionRoot> for InMemoryOutbox {
    async fn store(&self, outbox_entry: OutboxEntry) -> Result<(), Error> {
        let mut rows = self.rows();
        let id = rows.len() as i64 + 1;

        rows.push(InMemoryOutboxRow {
            stored_command: StoredCommand {
                id,
                command: outbox_entry.command,
                json: outbox_entry.json,
            },
            is_pending: true,
            error: None,
        });

        Ok(())
    }

    async fn next_pending(&self) -> Result<Option<StoredCommand>, Error> {
        let rows = self.rows();
        let oldest_pending = rows.iter().find(|row| row.is_pending);

        Ok(oldest_pending.map(|row| row.stored_command.clone()))
    }

    async fn mark_done(&self, id: i64) -> Result<(), Error> {
        self.finish(id, None);

        Ok(())
    }

    async fn mark_failed(&self, id: i64, error: &str) -> Result<(), Error> {
        self.finish(id, Some(error.to_string()));

        Ok(())
    }
}

/// `ChargeOrderCommand` → `charge_order`: the name of a command in the outbox and in JSON-RPC.
pub fn command_name<C>() -> String {
    let type_name = std::any::type_name::<C>();
    let type_name = type_name.split('<').next().unwrap_or(type_name);
    let type_name = type_name.rsplit("::").next().unwrap_or(type_name);
    let type_name = type_name.strip_suffix("Command").unwrap_or(type_name);

    let mut snake = String::new();

    for (position, letter) in type_name.chars().enumerate() {
        if letter.is_uppercase() && position > 0 {
            snake.push('_');
        }

        snake.extend(letter.to_lowercase());
    }

    snake
}

fn infrastructure(message: String) -> Error {
    InfrastructureError::from(anyhow::anyhow!(message)).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ChargeOrderCommand;

    #[test]
    fn command_name_is_the_snake_case_of_the_type_without_command() {
        assert_eq!(command_name::<ChargeOrderCommand>(), "charge_order");
    }
}
