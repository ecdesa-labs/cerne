//! # Cerne
//!
//! From Event Storming to Rust code. Every sticky note becomes an explicit block: whoever reads the code knows
//! right away which part of the flow they are in.
//!
//! | Sticky note | Concept | In Cerne |
//! |---|---|---|
//! | 🟦 | Command | [`application::Command`], which returns [`application::Executed`] |
//! | 🟨 | Aggregate / Entity | [`domain::entity`] and [`domain::aggregate`], which write [`domain::Entity`] and [`domain::Aggregate`]; the invariants go in [`domain::Validate`] |
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
    html_favicon_url = "https://raw.githubusercontent.com/ecdesa-labs/cerne/main/assets/cerne-favicon.svg",
    html_logo_url = "https://raw.githubusercontent.com/ecdesa-labs/cerne/main/assets/cerne-avatar-dark.svg"
)]

// The attributes of `cerne-macros` write `::cerne::..` paths: this makes them work inside this crate too.
extern crate self as cerne;

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

    #[doc(inline)]
    pub use crate::{business_rule, invariant, policy};
    pub use business_rules::*;
    pub use domain_events::*;

    /// The 🟨 entity: writes `impl Entity` and a `<Name>Constructor` for a struct with an `id: Option<<Name>Id>`.
    ///
    /// The constructor has every field but the id, which the repository decides on the first `save`. A field marked
    /// `#[skip_constructor]` stays out of it too, and starts at its `Default`: a status, for example. The invariants
    /// are not written: they go in an `impl Validate`, which `new` calls.
    ///
    /// ```
    /// use cerne::domain::{EnforcementResult, Entity, Validate, ValueObject, entity};
    ///
    /// # #[derive(Debug, Clone, PartialEq)]
    /// # struct OrderLineId(u64);
    /// # impl ValueObject for OrderLineId {
    /// #     type Constructor = u64;
    /// #     fn new(value: u64) -> EnforcementResult<Self> { Ok(Self(value)) }
    /// # }
    /// #[derive(Debug, Clone, Copy, Default, PartialEq)]
    /// enum OrderLineStatus {
    ///     #[default]
    ///     Open,
    ///     Shipped,
    /// }
    ///
    /// #[entity]
    /// struct OrderLine {
    ///     id: Option<OrderLineId>,
    ///     product: String,
    ///     #[skip_constructor]
    ///     status: OrderLineStatus,
    /// }
    ///
    /// impl Validate for OrderLine {
    ///     fn validate(self) -> EnforcementResult<Self> {
    ///         Ok(self)
    ///     }
    /// }
    ///
    /// let order_line = OrderLine::new(OrderLineConstructor {
    ///     product: "book".into(),
    /// })
    /// .unwrap();
    ///
    /// assert!(order_line.id().is_none());
    /// assert_eq!(order_line.status, OrderLineStatus::Open);
    /// ```
    pub use cerne_macros::entity;

    /// The 🟨 aggregate: everything [`entity`] writes, and `impl Aggregate`, so a `Repository` takes it.
    ///
    /// ```
    /// use cerne::application::Repository;
    /// use cerne::domain::{EnforcementResult, Validate, ValueObject, aggregate};
    ///
    /// # #[derive(Debug, Clone, PartialEq)]
    /// # struct OrderId(u64);
    /// # impl ValueObject for OrderId {
    /// #     type Constructor = u64;
    /// #     fn new(value: u64) -> EnforcementResult<Self> { Ok(Self(value)) }
    /// # }
    /// #[aggregate]
    /// struct Order {
    ///     id: Option<OrderId>,
    ///     total: u64,
    /// }
    ///
    /// impl Validate for Order {
    ///     fn validate(self) -> EnforcementResult<Self> {
    ///         Ok(self)
    ///     }
    /// }
    ///
    /// struct Ports {
    ///     orders: Box<dyn Repository<Order>>,
    /// }
    /// ```
    pub use cerne_macros::aggregate;

    /// The value object of one value: writes `TryFrom<u64> for OrderId`, through `ValueObject::new`, and
    /// `From<OrderId> for u64`, for a struct like `OrderId(u64)`.
    ///
    /// If the struct derives `Deserialize` or `Serialize` (in a `#[derive]` below the attribute), it also gets
    /// `#[serde(try_from = "u64", into = "u64")]`: in JSON it is the `u64` itself, and reading it back goes through
    /// `new`, so the invariants hold there too. `impl ValueObject`, with the invariants, is not written.
    ///
    /// ```
    /// use cerne::domain::{EnforcementResult, Invariants, ValueObject, invariant, value_object};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[value_object]
    /// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    /// struct OrderId(u64);
    ///
    /// impl ValueObject for OrderId {
    ///     type Constructor = u64;
    ///
    ///     fn new(value: u64) -> EnforcementResult<Self> {
    ///         let order_id_is_positive = value > 0;
    ///
    ///         Invariants::enforce([invariant!("order id is positive", order_id_is_positive)])?;
    ///
    ///         Ok(Self(value))
    ///     }
    /// }
    ///
    /// assert_eq!(u64::from(OrderId::try_from(7).unwrap()), 7);
    /// assert!(OrderId::try_from(0).is_err());
    /// assert!(serde_json::from_str::<OrderId>("0").is_err());
    /// ```
    pub use cerne_macros::value_object;

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
