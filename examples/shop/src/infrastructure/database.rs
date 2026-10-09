use cerne::Error;

/// The database of the application: write it on yours (a pool of `sqlx`, `diesel` or any other). The commands only
/// call `begin`, and pass the transaction to every repository and to the event outbox.
pub struct Database;

/// One transaction of the `Database`: the type every repository and the event outbox take.
pub struct Transaction;

impl Database {
    pub async fn begin(&self) -> Result<Transaction, Error> {
        todo!("open a transaction on your database")
    }
}

impl Transaction {
    /// Makes every write of the transaction permanent; dropping it without `commit` rolls them back.
    pub async fn commit(self) -> Result<(), Error> {
        todo!("commit the transaction on your database")
    }
}
