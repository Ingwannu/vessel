//! The database schema and its forward-only migrations.
//!
//! The version lives in `SQLite`'s `PRAGMA user_version`. Each entry of
//! [`MIGRATIONS`] (in `migrations`, with each version's SQL) moves the
//! schema up by one version; existing entries are never edited once
//! released, only appended to. A database newer than this build is refused
//! rather than guessed at.

#[cfg(test)]
mod background;
#[cfg(test)]
mod labels;
mod listing;
mod migrations;
mod workspace_registry;

// Diagnostic schema projection reads the canonical migration list only.
mod diagnostics;
pub(super) use diagnostics::{DiagnosticSchemaError, version as diagnostic_version};

use rusqlite::Connection;

#[cfg(test)]
use self::migrations::LABELS_SCHEMA;
#[cfg(test)]
pub(crate) use self::migrations::{CHECKPOINTS, CHECKPOINTS_SCHEMA};
use self::migrations::{INBOX, MIGRATIONS};
use crate::error::StoreError;

/// Bring the database to the newest schema.
pub(crate) fn migrate(connection: &mut Connection) -> Result<(), StoreError> {
    let found: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let supported = i64::try_from(MIGRATIONS.len()).unwrap_or(i64::MAX);
    if found > supported {
        return Err(StoreError::NewerSchema { found, supported });
    }
    let done = usize::try_from(found).unwrap_or(0);
    let pending = MIGRATIONS.get(done..).unwrap_or_default();
    if pending.is_empty() {
        return Ok(());
    }
    let transaction = connection.transaction()?;
    for migration in pending {
        transaction.execute_batch(migration)?;
    }
    if pending.contains(&INBOX) {
        crate::inbox::backfill(&transaction)?;
    }
    transaction.pragma_update(None, "user_version", supported)?;
    transaction.commit()?;
    tracing::info!(
        from = found,
        to = supported,
        "migrated the event log schema"
    );
    Ok(())
}

/// A rollback fixture targets the inbox entry, independent of later appends.
/// Only tests may rewind; production migration stays forward-only.
#[cfg(test)]
pub(crate) fn replay_inbox_after_rollback(connection: &mut Connection) {
    let before = MIGRATIONS
        .iter()
        .position(|migration| *migration == INBOX)
        .unwrap();
    connection
        .pragma_update(None, "user_version", i64::try_from(before).unwrap())
        .unwrap();
    migrate(connection).unwrap();
}

/// Settings every connection needs.
pub(crate) fn configure(connection: &Connection) -> Result<(), StoreError> {
    // Wait for a lock instead of failing at once; only the checkpoint and
    // migrations ever hold one long.
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    connection.pragma_update(None, "foreign_keys", true)?;
    Ok(())
}

/// Settings for the writer connection: WAL so readers never block on it,
/// and `synchronous=NORMAL` so a commit does not `fsync` (WAL syncs at
/// checkpoints; a power cut may lose the last commits, a crash does not).
pub(crate) fn configure_writer(connection: &Connection) -> Result<(), StoreError> {
    configure(connection)?;
    let mode: String =
        connection.pragma_update_and_check(None, "journal_mode", "wal", |row| row.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        tracing::warn!(%mode, "the event log could not switch to WAL mode");
    }
    connection.pragma_update(None, "synchronous", "normal")?;
    Ok(())
}

/// Lets staged tests inspect the actual appended count without duplicating it.
#[cfg(test)]
pub(crate) const fn migration_count() -> usize {
    MIGRATIONS.len()
}
