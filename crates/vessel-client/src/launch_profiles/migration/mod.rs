//! App-supplied compatibility inputs to ordinary host CAS; no preference I/O.
//!
//! The app owns markers and in-flight ownership keyed by `NodeId`. This module owns
//! bounded catalogue resolution, append-only planning and reconciliation.
//! A completion receipt may be marked only after the host result is known.

mod catalogue;
mod plan;
mod worker;

pub use catalogue::{LegacyCatalogueState, MigrationCatalogue, MigrationCatalogues};
pub use plan::{LegacyMigrationPlan, build_migration_plan};
pub use worker::{LegacyMigrationCompletion, LegacyMigrationReceipt};

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod plan_tests;
