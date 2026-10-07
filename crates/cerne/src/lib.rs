//! # Cerne
//!
//! Do Event Storming ao código Rust. Cada post-it vira um bloco explícito: quem olha o código sabe na hora em que
//! parte do fluxo está.
//!
//! | Post-it | Conceito | No Cerne |
//! |---|---|---|
//! | 🟦 | Command | [`application::Command`], que devolve [`application::Executed`] |
//! | 🟨 | Aggregate / Entity | [`domain::Entity`] e [`domain::Aggregate`] |
//! | — | Value Object | [`domain::ValueObject`], que também é o tipo do id de toda entidade |
//! | 🟧 | Domain Event | [`domain::DomainEvent`] |
//! | 🟪 | Policy | [`domain::Policy`] + [`domain::Policies`], executadas por um [`application::PolicyProcessor`] ou guardadas numa [`application::Outbox`] |
//! | 🩷 | External System (Port) | [`application::Repository`] e os ports da aplicação |
//! | 🟩 | Read Model / Query | [`application::Query`], que devolve um [`application::ReadModel`] |
//! | — | Invariantes | [`domain::Invariant`] + [`domain::Invariants`] |
//! | — | Regras de negócio | [`domain::BusinessRule`] + [`domain::BusinessRules`] |
//!
//! Os erros seguem três categorias, reunidas em [`Error`]: [`DomainError`], [`ApplicationError`] e
//! [`InfrastructureError`].
//!
//! O banco entra pelos adapters SQL: [`sqlite`] (feature padrão, também em memória) e `postgres` (feature
//! `postgres`). Os dois têm a mesma API, e trocar um pelo outro é trocar o adapter. O HTTP entra pela feature
//! `axum`: [`Error`] vira resposta REST, e `http::jsonrpc` atende JSON-RPC.
//!
//! Cada trait traz um exemplo. O tutorial completo, que percorre uma aplicação de ponta a ponta (o `examples/rde`),
//! está no README do repositório.

mod business_rules;
mod commands;
mod domain_events;
mod entities;
mod errors;
#[cfg(feature = "axum")]
pub mod http;
mod invariants;
mod outbox;
mod policies;
mod policy_processors;
mod queries;
mod repositories;
#[cfg(any(feature = "sqlite", feature = "postgres"))]
mod sql;
mod value_objects;

pub use async_trait::async_trait;
pub use errors::{ApplicationError, DomainError, Error, InfrastructureError};

/// Pure and synchronous: entities, value objects, events, invariants, business rules and policies. No IO happens here.
pub mod domain {
    use super::*;

    pub use business_rules::*;
    pub use domain_events::*;
    pub use entities::*;
    pub use errors::{DomainError, EnforcementResult};
    pub use invariants::*;
    pub use policies::*;
    pub use value_objects::*;
}

/// Asynchronous: commands, queries, the ports they use and the processor that runs the commands policies return.
pub mod application {
    use super::*;

    pub use commands::*;
    pub use outbox::*;
    pub use policy_processors::*;
    pub use queries::*;
    pub use repositories::*;
}

/// SQLite: a file, or a database in memory for tests and prototypes. Never a `Vec` pretending to be a repository.
#[cfg(feature = "sqlite")]
pub mod sqlite {
    crate::sql::sql_adapters! {
        database: SqliteDatabase,
        outbox: SqliteOutbox,
        db: sqlx::Sqlite,
        pool: sqlx::SqlitePool,
        pool_options: sqlx::sqlite::SqlitePoolOptions,
        row: sqlx::sqlite::SqliteRow,
        // Takes the write lock at the start: a transaction that reads and then writes waits for the other writers,
        // instead of failing with "database is locked" when one of them commits in between.
        begin: "BEGIN IMMEDIATE",
        lock_next_pending: "",
    }

    impl SqliteDatabase {
        /// A database in memory, gone when the process ends. It lives on a single connection that is never closed,
        /// so a transaction holds the whole database until it commits or rolls back.
        pub async fn in_memory() -> Result<Self, Error> {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .min_connections(1)
                .idle_timeout(None)
                .max_lifetime(None)
                .connect("sqlite::memory:")
                .await
                .map_err(infrastructure)?;

            Ok(Self(Connection::Pool(pool)))
        }
    }
}

/// Postgres, with the same API as [`sqlite`]: moving an application is swapping `Sqlite*` for `Postgres*` and the
/// migrations.
#[cfg(feature = "postgres")]
pub mod postgres {
    crate::sql::sql_adapters! {
        database: PostgresDatabase,
        outbox: PostgresOutbox,
        db: sqlx::Postgres,
        pool: sqlx::PgPool,
        pool_options: sqlx::postgres::PgPoolOptions,
        row: sqlx::postgres::PgRow,
        begin: "BEGIN",
        lock_next_pending: " FOR UPDATE SKIP LOCKED",
    }
}
