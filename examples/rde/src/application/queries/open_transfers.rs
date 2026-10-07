use crate::application::read_models::open_transfers::OpenTransfers;
use crate::domain::value_objects::address::Address;
use crate::domain::value_objects::tx_hash::TxHash;
use crate::ports::Ports;
use cerne::application::Query;
use cerne::domain::ValueObject;
use cerne::sqlite::column;
use cerne::{Error, async_trait};
use serde::Deserialize;

/// No actor: `CreateTransferCommand` reads it, because a sender has one open transfer at a time. Two open
/// transfers would carry the same nonce, and the chain takes only one of them.
#[derive(Deserialize)]
pub struct OpenTransfersQuery {
    pub sender: Address,
}

#[async_trait]
impl Query<Ports> for OpenTransfersQuery {
    type ReadModel = OpenTransfers;

    async fn execute(&self, ports: &Ports) -> Result<OpenTransfers, Error> {
        // --- Ports -----------------------------------------------------------

        let select = sqlx::query(
            "SELECT tx_hash FROM transfers
             WHERE sender = $1 AND NOT chained
             ORDER BY nonce",
        )
        .bind(self.sender.to_string());

        let rows = ports.database.fetch_all(select).await?;

        // --- Read model ------------------------------------------------------

        let transfers = rows
            .iter()
            .map(|row| Ok(TxHash::new(column(row, "tx_hash")?)?))
            .collect::<Result<Vec<_>, Error>>()?;

        let open_transfers = OpenTransfers {
            sender: self.sender.clone(),
            transfers,
        };

        Ok(open_transfers)
    }
}
