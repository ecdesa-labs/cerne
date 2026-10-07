//! The SQL adapters: one module per database, written once by `sql_adapters!` (D31).
//!
//! The SQL is the same in both databases (`$1` placeholders and `RETURNING` work in SQLite and Postgres); only the
//! migrations, written by the application, differ.

use crate::errors::{Error, InfrastructureError};

pub(crate) fn infrastructure(error: impl Into<anyhow::Error>) -> Error {
    InfrastructureError::from(error.into()).into()
}

macro_rules! sql_adapters {
    (
        database: $database:ident,
        outbox: $outbox:ident,
        db: $db:ty,
        pool: $pool:ty,
        pool_options: $pool_options:ty,
        row: $row:ty,
        begin: $begin:literal,
        lock_next_pending: $lock_next_pending:literal,
    ) => {
        use crate::errors::Error;
        use crate::outbox::{Outbox, OutboxEntry, StoredCommand};
        use crate::sql::infrastructure;
        use async_trait::async_trait;
        use sqlx::Row;
        use std::sync::Arc;
        use tokio::sync::Mutex;

        type Query<'q> = sqlx::query::Query<'q, $db, <$db as sqlx::Database>::Arguments<'q>>;

        /// The database: the pool of connections, or a transaction open on one of them.
        ///
        /// A clone shares the same pool, or the same transaction: every repository built from a clone of a
        /// transaction writes in it, and `commit` makes all of it permanent.
        #[derive(Clone)]
        pub struct $database(Connection);

        #[derive(Clone)]
        enum Connection {
            Pool($pool),
            Transaction(Arc<Mutex<Option<sqlx::Transaction<'static, $db>>>>),
        }

        impl $database {
            /// Connects to `url`; `max_connections` is the size of the pool.
            pub async fn connect(url: &str, max_connections: u32) -> Result<Self, Error> {
                let pool = <$pool_options>::new()
                    .max_connections(max_connections)
                    .connect(url)
                    .await
                    .map_err(infrastructure)?;

                Ok(Self(Connection::Pool(pool)))
            }

            /// Runs the migrations not run yet, like `sqlx::migrate!()` (the `migrations/` folder of the application).
            pub async fn migrate(&self, migrator: &sqlx::migrate::Migrator) -> Result<(), Error> {
                let Connection::Pool(pool) = &self.0 else {
                    return Err(infrastructure(anyhow::anyhow!(
                        "migrations run on the pool, not inside a transaction"
                    )));
                };

                migrator.run(pool).await.map_err(infrastructure)?;

                Ok(())
            }

            /// Opens a transaction: a clone of what `begin` returns writes in it.
            pub async fn begin(&self) -> Result<Self, Error> {
                let Connection::Pool(pool) = &self.0 else {
                    return Err(infrastructure(anyhow::anyhow!("a transaction is already open")));
                };

                let transaction = pool.begin_with($begin).await.map_err(infrastructure)?;

                Ok(Self(Connection::Transaction(Arc::new(Mutex::new(Some(transaction))))))
            }

            /// Makes every write of the transaction permanent.
            pub async fn commit(&self) -> Result<(), Error> {
                let Connection::Transaction(transaction) = &self.0 else {
                    return Err(infrastructure(anyhow::anyhow!("there is no transaction to commit")));
                };

                let transaction = transaction.lock().await.take().ok_or_else(|| {
                    infrastructure(anyhow::anyhow!("the transaction was already committed"))
                })?;

                transaction.commit().await.map_err(infrastructure)?;

                Ok(())
            }

            /// Runs an `INSERT`, `UPDATE` or `DELETE`; returns how many rows it touched.
            pub async fn execute(&self, query: Query<'_>) -> Result<u64, Error> {
                let result = match &self.0 {
                    Connection::Pool(pool) => query.execute(pool).await,
                    Connection::Transaction(transaction) => {
                        let mut transaction = transaction.lock().await;
                        query.execute(&mut **open(&mut transaction)?).await
                    }
                };

                Ok(result.map_err(infrastructure)?.rows_affected())
            }

            pub async fn fetch_one(&self, query: Query<'_>) -> Result<$row, Error> {
                let row = match &self.0 {
                    Connection::Pool(pool) => query.fetch_one(pool).await,
                    Connection::Transaction(transaction) => {
                        let mut transaction = transaction.lock().await;
                        query.fetch_one(&mut **open(&mut transaction)?).await
                    }
                };

                row.map_err(infrastructure)
            }

            pub async fn fetch_optional(&self, query: Query<'_>) -> Result<Option<$row>, Error> {
                let row = match &self.0 {
                    Connection::Pool(pool) => query.fetch_optional(pool).await,
                    Connection::Transaction(transaction) => {
                        let mut transaction = transaction.lock().await;
                        query.fetch_optional(&mut **open(&mut transaction)?).await
                    }
                };

                row.map_err(infrastructure)
            }

            pub async fn fetch_all(&self, query: Query<'_>) -> Result<Vec<$row>, Error> {
                let rows = match &self.0 {
                    Connection::Pool(pool) => query.fetch_all(pool).await,
                    Connection::Transaction(transaction) => {
                        let mut transaction = transaction.lock().await;
                        query.fetch_all(&mut **open(&mut transaction)?).await
                    }
                };

                rows.map_err(infrastructure)
            }
        }

        /// Reads the column `name` of a row; a missing column or a wrong type is an `InfrastructureError`.
        pub fn column<T>(row: &$row, name: &str) -> Result<T, Error>
        where
            T: for<'r> sqlx::Decode<'r, $db> + sqlx::Type<$db>,
        {
            row.try_get(name).map_err(infrastructure)
        }

        fn open<'t>(
            transaction: &'t mut Option<sqlx::Transaction<'static, $db>>,
        ) -> Result<&'t mut sqlx::Transaction<'static, $db>, Error> {
            transaction
                .as_mut()
                .ok_or_else(|| infrastructure(anyhow::anyhow!("the transaction was already committed")))
        }

        /// The outbox in the table `cerne_outbox`, which a migration of the application creates:
        ///
        /// `id` (integer key decided by the database), `command` and `json` (text), `status` (text, `'pending'` by
        /// default, then `'done'` or `'failed'`) and `error` (text, null until the command fails).
        pub struct $outbox {
            database: $database,
        }

        impl $outbox {
            pub fn new(database: $database) -> Self {
                Self { database }
            }
        }

        #[async_trait]
        impl<Ports: 'static> Outbox<Ports> for $outbox {
            async fn store(&self, outbox_entry: OutboxEntry) -> Result<(), Error> {
                let insert = sqlx::query("INSERT INTO cerne_outbox (command, json) VALUES ($1, $2)")
                    .bind(outbox_entry.command)
                    .bind(outbox_entry.json);

                self.database.execute(insert).await?;

                Ok(())
            }

            async fn next_pending(&self) -> Result<Option<StoredCommand>, Error> {
                let select = sqlx::query(concat!(
                    "SELECT id, command, json FROM cerne_outbox WHERE status = 'pending' ORDER BY id LIMIT 1",
                    $lock_next_pending
                ));

                let Some(row) = self.database.fetch_optional(select).await? else {
                    return Ok(None);
                };

                let stored_command = StoredCommand {
                    id: row.try_get("id").map_err(infrastructure)?,
                    command: row.try_get("command").map_err(infrastructure)?,
                    json: row.try_get("json").map_err(infrastructure)?,
                };

                Ok(Some(stored_command))
            }

            async fn mark_done(&self, id: i64) -> Result<(), Error> {
                let update = sqlx::query("UPDATE cerne_outbox SET status = 'done' WHERE id = $1").bind(id);

                self.database.execute(update).await?;

                Ok(())
            }

            async fn mark_failed(&self, id: i64, error: &str) -> Result<(), Error> {
                let update = sqlx::query("UPDATE cerne_outbox SET status = 'failed', error = $2 WHERE id = $1")
                    .bind(id)
                    .bind(error.to_string());

                self.database.execute(update).await?;

                Ok(())
            }
        }
    };
}

pub(crate) use sql_adapters;
