//! ADR 0077 staged SQL boundary. Wiring moves operations to the existing writer.
//!
//! The schema is assigned centrally; never install a fake migration in a slot
//! reserved for another feature. migrate() and the existing migration list stay
//! unchanged until the launch-profile predecessor is available.

mod journal;
#[cfg(test)]
mod journal_tests;
mod records;

#[cfg(test)]
mod tests;

use crate::StoreError;
use rusqlite::Connection;

/// Assigned checkpoint migration; the intervening slot belongs to launch profiles.
const CHECKPOINTS_SCHEMA: usize = super::schema::migrations::TITLE_ORIGIN_SCHEMA.saturating_add(2);
const CHECKPOINTS: &str = include_str!("schema.sql");

fn install_staged(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch(CHECKPOINTS)?;
    Ok(())
}
