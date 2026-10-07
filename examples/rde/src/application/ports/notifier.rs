use cerne::{Error, async_trait};

/// External system "Notificador": delivers a message to the owner of a wallet (push, e-mail, SMS).
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(&self, wallet: &str, message: &str) -> Result<(), Error>;
}
