use crate::errors::{Error, InfrastructureError};
use async_trait::async_trait;
use serde::Serialize;

/// The port of the event outbox: every event a command produces, stored in the same transaction as the aggregate that
/// produced it.
///
/// Cerne only writes to it. Reading the table back (to run an event again, to audit) is up to the application, and so
/// are the table and its adapter; `event_outbox` is the suggested name.
///
/// ```
/// use cerne::application::{EventOutbox, OutboxEntry};
/// use cerne::{Error, async_trait};
/// use std::sync::Mutex;
///
/// #[derive(Default)]
/// struct InMemoryEventOutbox {
///     outbox_entries: Mutex<Vec<OutboxEntry>>,
/// }
///
/// #[async_trait]
/// impl EventOutbox for InMemoryEventOutbox {
///     async fn store(&self, outbox_entry: OutboxEntry) -> Result<(), Error> {
///         self.outbox_entries.lock().unwrap().push(outbox_entry);
///
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait EventOutbox: Send + Sync {
    /// Stores one event, in the transaction of the command that produced it.
    async fn store(&self, outbox_entry: OutboxEntry) -> Result<(), Error>;
}

/// An event as the outbox stores it: its name and its fields in JSON.
#[derive(Debug, Clone, PartialEq)]
pub struct OutboxEntry {
    /// `OrderPlaced` → `"order_placed"`.
    pub event: String,
    pub json: String,
}

impl OutboxEntry {
    /// `OutboxEntry::new(&order_placed)?`: the name of the event's type in snake case, and the event in JSON.
    pub fn new<E: Serialize>(event: &E) -> Result<Self, Error> {
        let event_name = event_name::<E>();

        let json = serde_json::to_string(event).map_err(|error| {
            InfrastructureError::from(anyhow::anyhow!("cannot store {event_name} in the outbox: {error}"))
        })?;

        Ok(Self {
            event: event_name,
            json,
        })
    }
}

/// `OrderPlaced` → `order_placed`.
fn event_name<E>() -> String {
    let type_name = std::any::type_name::<E>();
    let type_name = type_name.split('<').next().unwrap_or(type_name);
    let type_name = type_name.rsplit("::").next().unwrap_or(type_name);

    let mut snake = String::new();

    for (position, letter) in type_name.chars().enumerate() {
        if letter.is_uppercase() && position > 0 {
            snake.push('_');
        }

        snake.extend(letter.to_lowercase());
    }

    snake
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct OrderPlaced {
        order_id: u64,
    }

    #[test]
    fn outbox_entry_is_the_snake_case_of_the_event_and_its_json() {
        let order_placed = OrderPlaced { order_id: 7 };

        assert_eq!(
            OutboxEntry::new(&order_placed).unwrap(),
            OutboxEntry {
                event: "order_placed".into(),
                json: r#"{"order_id":7}"#.into()
            }
        );
    }
}
