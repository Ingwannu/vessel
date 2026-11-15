//! Retention transfers checkpoint accounting atomically; unfinished work stays pinned.
use super::{garbage, records::CheckpointStoreError};
use crate::{StoreError, codec, config::Retention, log, marks};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use vessel_proto::{SessionId, checkpoint::CheckpointError};

/// Caller owns the writer/startup transaction, including session deletion.
pub(crate) fn prune(
    connection: &Connection,
    retention: &Retention,
    at: DateTime<Utc>,
) -> Result<Vec<SessionId>, StoreError> {
    // Preserve the established board retention order and cap. Pinned checkpoint
    // work beyond that cap remains additional retained history until settled.
    let mut statement = connection.prepare(&format!(
        "SELECT id FROM sessions WHERE live=0 AND ended_at IS NOT NULL {} LIMIT -1 OFFSET ?1",
        log::KEPT_FIRST,
    ))?;
    let raw = statement
        .query_map([retention.max_ended_sessions], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    let mut removed = Vec::new();
    for raw in raw {
        let session = codec::session_id_from_sql(&raw)?;
        match garbage::retire_session(connection, session, at) {
            Ok(()) => {
                marks::delete(connection, session)?;
                removed.push(session);
            }
            Err(CheckpointStoreError::Refused(
                CheckpointError::Busy | CheckpointError::RecoveryPending,
            )) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(removed)
}

/// Startup fences all generations and interrupted restores before retention runs.
/// No filesystem cleanup or Ready promotion happens on opening the database.
pub(crate) fn startup(connection: &mut Connection) -> Result<(), StoreError> {
    let original: i64 = connection.pragma_query_value(None, "synchronous", |row| row.get(0))?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    let result: Result<(), StoreError> = (|| {
        let transaction = connection.transaction()?;
        super::journal::invalidate_candidates(&transaction)?;
        // The node lock proves old process I/O cannot still own these rows.
        // Future live captures remain active and never enter this startup scan.
        let _fenced = transaction.execute(
            "UPDATE checkpoint_captures SET worker='orphaned' WHERE worker='active'",
            [],
        )?;
        let _fenced = transaction.execute(
            "UPDATE checkpoint_allocations SET phase='orphaned' WHERE phase='reserved'",
            [],
        )?;
        while super::recovery::pending(&transaction)?
            == vessel_proto::checkpoint::CHECKPOINT_PAGE_ENTRIES
        {}
        transaction.commit()?;
        Ok(())
    })();
    let reset = connection.pragma_update(None, "synchronous", original);
    match (result, reset) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests;
