//! ADR 0077 staged SQL boundary. Wiring moves operations to the existing writer.
//!
//! The schema appends directly after this base. The integrator re-points the
//! one schema constant at merge. The existing migration runner is unchanged.

mod journal;
#[cfg(test)]
mod journal_tests;
mod paths;
mod plans;
mod publication;
mod records;
mod recovery;

#[cfg(test)]
mod tests;

use crate::StoreError;
use rusqlite::Connection;

use super::schema::{CHECKPOINTS, CHECKPOINTS_SCHEMA};

fn install_staged(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch(CHECKPOINTS)?;
    Ok(())
}
