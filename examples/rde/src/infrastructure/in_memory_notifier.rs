use crate::application::ports::notifier::Notifier;
use cerne::{Error, async_trait};
use std::sync::{Arc, Mutex};

/// A message delivered to the owner of a wallet.
#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub wallet: String,
    pub message: String,
}

/// Every notification goes to an inbox shared with whoever built the notifier, so they can read it.
pub struct InMemoryNotifier {
    inbox: Arc<Mutex<Vec<Notification>>>,
}

impl InMemoryNotifier {
    pub fn new(inbox: Arc<Mutex<Vec<Notification>>>) -> Self {
        Self { inbox }
    }
}

#[async_trait]
impl Notifier for InMemoryNotifier {
    async fn notify(&self, wallet: &str, message: &str) -> Result<(), Error> {
        let notification = Notification {
            wallet: wallet.to_string(),
            message: message.to_string(),
        };

        self.inbox.lock().unwrap().push(notification);

        Ok(())
    }
}
