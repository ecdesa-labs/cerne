//! # Cerne
//!
//! From Event Storming to Rust code. Every sticky note becomes an explicit block: whoever reads the code knows
//! right away which part of the flow they are in.
//!
//! | Sticky note | Concept | In Cerne |
//! |---|---|---|
//! | 🟦 | Command | [`application::Command`], which returns [`application::Executed`] |
//! | 🟨 | Aggregate / Entity | [`domain::Entity`] and [`domain::Aggregate`] |
//! | — | Value Object | [`domain::ValueObject`], also the type of every entity id |
//! | 🟧 | Domain Event | [`domain::DomainEvent`] |
//! | 🟪 | Policy | [`domain::Policy`] + [`domain::Policies`], run by an [`application::PolicyProcessor`] or stored in an [`application::Outbox`] |
//! | 🩷 | External System (Port) | [`application::Repository`] and the ports of the application |
//! | 🟩 | Read Model / Query | [`application::Query`], which returns an [`application::ReadModel`] |
//! | — | Invariants | [`domain::Invariant`] + [`domain::Invariants`] |
//! | — | Business rules | [`domain::BusinessRule`] + [`domain::BusinessRules`] |
//!
//! Errors fall into three categories, gathered in [`Error`]: [`DomainError`], [`ApplicationError`] and
//! [`InfrastructureError`].
//!
//! The database comes in through the SQL adapters: `sqlite` (default feature, also in memory) and `postgres`
//! (feature `postgres`). Both have the same API, and switching is switching the adapter. HTTP comes in through the
//! feature `axum`: [`Error`] becomes a REST answer, and `http::jsonrpc` serves JSON-RPC.
//!
//! Every trait carries an example. The `cerne` command (crate `cerne-cli`) creates a project laid out like the
//! board and generates each sticky note; the README walks through one from start to finish.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/ecdesa-labs/cerne/main/assets/cerne-favicon.svg"
)]

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
