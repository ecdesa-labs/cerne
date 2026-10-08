use crate::commands::Command;
use crate::domain_events::DomainEvent;
use crate::errors::{Error, InfrastructureError};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Arc;
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
/// with the `postgres` feature, `cerne::postgres::PostgresOutbox`.
#[async_trait]
pub trait Outbox<Ports: 'static>: Send + Sync {
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
        events: Vec<Box<dyn DomainEvent<Ports>>>,
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

/// Ports that can open a database transaction: the ports `begin` returns write every repository and the outbox in
/// that transaction, and `commit` makes it permanent. Dropping them without `commit` rolls everything back.
///
/// External systems (a blockchain, a notifier) are not part of the transaction: `begin` hands the same adapters to
/// the new ports. That is why calls to them belong in commands fired by policies, which the outbox runs at least once.
#[async_trait]
pub trait TransactionalPorts: Sized + Send + Sync + 'static {
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

type Decode<Ports> = fn(&str) -> serde_json::Result<Box<dyn Command<Ports, Output = ()>>>;

/// Turns a row of the outbox back into the command it stores: every command a policy fires is registered here.
///
/// ```ignore
/// let command_registry = CommandRegistry::new()
///     .register::<ReserveStockCommand>()
///     .register::<ChargeOrderCommand>();
/// ```
pub struct CommandRegistry<Ports> {
    decoders: HashMap<String, Decode<Ports>>,
}

impl<Ports: 'static> CommandRegistry<Ports> {
    pub fn new() -> Self {
        Self {
            decoders: HashMap::new(),
        }
    }

    pub fn register<C>(mut self) -> Self
    where
        C: Command<Ports, Output = ()> + DeserializeOwned + 'static,
    {
        fn decode<Ports, C>(json: &str) -> serde_json::Result<Box<dyn Command<Ports, Output = ()>>>
        where
            C: Command<Ports, Output = ()> + DeserializeOwned + 'static,
        {
            Ok(Box::new(serde_json::from_str::<C>(json)?))
        }

        self.decoders
            .insert(command_name::<C>(), decode::<Ports, C>);

        self
    }

    pub fn decode(
        &self,
        stored_command: &StoredCommand,
    ) -> Result<Box<dyn Command<Ports, Output = ()>>, Error> {
        let decode = self.decoders.get(&stored_command.command).ok_or_else(|| {
            infrastructure(format!(
                "{} is not in the command registry",
                stored_command.command
            ))
        })?;

        let command = decode(&stored_command.json).map_err(|error| {
            infrastructure(format!("cannot read {}: {error}", stored_command.command))
        })?;

        Ok(command)
    }
}

impl<Ports: 'static> Default for CommandRegistry<Ports> {
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
pub struct OutboxPolicyProcessor<Ports> {
    ports: Arc<Ports>,
    command_registry: Arc<CommandRegistry<Ports>>,
}

impl<Ports: TransactionalPorts> OutboxPolicyProcessor<Ports> {
    pub fn new(ports: Arc<Ports>, command_registry: CommandRegistry<Ports>) -> Self {
        Self {
            ports,
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
        let transaction = self.ports.begin().await?;

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
                let failure = self.ports.begin().await?;

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

    async fn execute(
        &self,
        stored_command: &StoredCommand,
        transaction: &Ports,
    ) -> Result<(), Error> {
        let command = self.command_registry.decode(stored_command)?;

        let execution = command.execute(transaction).await?;

        transaction.outbox().send_events(execution.events).await?;

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
