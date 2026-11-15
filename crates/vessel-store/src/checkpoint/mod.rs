//! ADR 0077 typed storage boundary; authentication and filesystem work stay on the node.
//!
//! The schema appends directly after this base. The integrator re-points the
//! one schema constant at merge. The existing migration runner is unchanged.

mod allocation;
mod changes;
mod completed;
mod garbage;
mod journal;

mod list;
pub(crate) mod metadata;
mod orphans;
mod paths;
mod plans;
mod private_orphans;
mod publication;
mod query;
mod quota;
mod records;
mod recovery;
pub(crate) mod retention;
mod rotation;

mod staging;
mod startup;
pub(crate) use startup::replay_metadata;
mod write;

pub use allocation::Allocation;
pub(crate) use garbage::retire_session;
pub use garbage::{Garbage, Object as GarbageObject};
pub use orphans::{OrphanedCapture, OrphanedPage};
pub use paths::{Entry as PathIntent, Phase as PathPhase};
pub use query::{OperationPage, Selected};
pub use records::{CheckpointStoreError, Operation, Plan};
pub use write::CheckpointWrite;
pub(crate) use write::apply;

#[cfg(test)]
use crate::StoreError;
#[cfg(test)]
use rusqlite::Connection;

#[cfg(test)]
use super::schema::{CHECKPOINTS, CHECKPOINTS_SCHEMA};

#[cfg(test)]
fn install_staged(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch(CHECKPOINTS)?;
    Ok(())
}

#[cfg(test)]
mod journal_tests;

#[cfg(test)]
mod rotation_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod writer_tests;

#[cfg(test)]
mod metadata_tests;
