//! ADR 0077 staged SQL boundary. Wiring moves operations to the existing writer.
//!
//! The schema appends directly after this base. The integrator re-points the
//! one schema constant at merge. The existing migration runner is unchanged.

mod allocation;
mod changes;
mod garbage;
mod journal;
#[cfg(test)]
mod journal_tests;
mod list;
pub(crate) mod metadata;
mod paths;
mod plans;
mod publication;
mod quota;
mod records;
mod recovery;
pub(crate) mod retention;
mod write;

pub(crate) use garbage::{Object as GarbageObject, retire_session};
pub use records::CheckpointStoreError;
pub(crate) use write::{CheckpointWrite, apply};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod writer_tests;

use crate::StoreError;
use rusqlite::Connection;

use super::schema::{CHECKPOINTS, CHECKPOINTS_SCHEMA};

fn install_staged(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch(CHECKPOINTS)?;
    Ok(())
}

#[cfg(test)]
mod metadata_tests;
