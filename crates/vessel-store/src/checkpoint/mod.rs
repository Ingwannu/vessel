//! ADR 0077 staged SQL boundary. Wiring moves operations to the existing writer.
//!
//! The schema appends directly after this base. The integrator re-points the
//! one schema constant at merge. The existing migration runner is unchanged.

mod changes;
mod garbage;
mod journal;
#[cfg(test)]
mod journal_tests;
mod list;
mod paths;
mod plans;
mod publication;
mod records;
mod recovery;
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
