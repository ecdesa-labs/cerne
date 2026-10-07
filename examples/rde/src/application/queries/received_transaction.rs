use crate::application::read_models::received_transaction::ReceivedTransaction;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::application::Query;
use cerne::{Error, async_trait};
use serde::Deserialize;

/// No actor: `eth_sendRawTransaction` reads it before anything else. A node answers a transaction it already has
/// with the same hash, and MetaMask sends one again: "Speed up" on a cancellation sends the same bytes.
#[derive(Deserialize)]
pub struct ReceivedTransactionQuery {
    pub tx_hash: TxHash,
}

#[async_trait]
impl Query<Ports> for ReceivedTransactionQuery {
    type ReadModel = ReceivedTransaction;

    async fn execute(&self, ports: &Ports) -> Result<ReceivedTransaction, Error> {
        // --- Ports -----------------------------------------------------------

        let select = sqlx::query(
            "SELECT tx_hash FROM transfers WHERE tx_hash = $1 OR cancellation_tx_hash = $1",
        )
        .bind(self.tx_hash.to_string());

        let row = ports.database.fetch_optional(select).await?;

        // --- Read model ------------------------------------------------------

        let received_transaction = ReceivedTransaction {
            tx_hash: self.tx_hash.clone(),
            received: row.is_some(),
        };

        Ok(received_transaction)
    }
}
