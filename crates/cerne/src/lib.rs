//! # Cerne
//!
//! Do Event Storming ao código Rust. Cada post-it vira um bloco explícito: quem olha o código sabe na hora em que
//! parte do fluxo está.
//!
//! | Post-it | Conceito | No Cerne |
//! |---|---|---|
//! | 🟦 | Command | [`application::Command`], que devolve [`application::Executed`] |
//! | 🟨 | Aggregate / Entity | [`domain::Entity`] e [`domain::Aggregate`] |
//! | 🟧 | Domain Event | [`domain::DomainEvent`] |
//! | 🟪 | Policy | [`domain::Policy`] + [`domain::Policies`], executadas por um [`application::PolicyProcessor`] |
//! | 🩷 | External System (Port) | [`application::Repository`] e os ports da aplicação |
//! | — | Invariantes | [`domain::Invariant`] + [`domain::Invariants`] |
//! | — | Regras de negócio | [`domain::BusinessRule`] + [`domain::BusinessRules`] |
//!
//! Os erros seguem três categorias, reunidas em [`Error`]: [`DomainError`], [`ApplicationError`] e
//! [`InfrastructureError`].
//!
//! Cada trait traz um exemplo. O tutorial completo, que percorre uma aplicação de ponta a ponta (o `examples/rde`),
//! está no README do repositório.

mod business_rules;
mod commands;
mod domain_events;
mod entities;
mod errors;
mod invariants;
mod policies;
mod policy_processors;
mod repositories;

pub use async_trait::async_trait;
pub use errors::{ApplicationError, DomainError, Error, InfrastructureError};

/// Pure and synchronous: entities, events, invariants, business rules and policies. No IO happens here.
pub mod domain {
    use super::*;

    pub use business_rules::*;
    pub use domain_events::*;
    pub use entities::*;
    pub use errors::{DomainError, EnforcementResult};
    pub use invariants::*;
    pub use policies::*;
}

/// Asynchronous: commands, the ports they use and the processor that runs the commands policies return.
pub mod application {
    use super::*;

    pub use commands::*;
    pub use policy_processors::*;
    pub use repositories::*;
}
