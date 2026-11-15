//! Retiring metadata never releases archive capacity before physical cleanup.
//!
//! [Decision Log]
//! - Purpose: session deletion must remove unpinned images without breaking quota.
//! - Constraint: files cannot be removed inside a `SQLite` transaction atomically.
//! - Choice: transfer the reservation into durable garbage independent of the
//!   session FK; only a verified filesystem cleanup acknowledgement releases it.
//! - Cost: interrupted cleanup conservatively retains capacity until reconciliation.
use super::records::CheckpointStoreError as Error;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use vessel_proto::{
    SessionId, WorktreeId,
    checkpoint::{CHECKPOINT_PAGE_ENTRIES, CaptureId, CheckpointError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
/// Typed private object awaiting physical cleanup; independent of session FKs.
pub enum Object {
    /// A capture and all private allocations it owned.
    Capture {
        /// Worktree storage namespace.
        worktree: WorktreeId,
        /// Capture storage namespace.
        capture: CaptureId,
    },
    /// An expired unconsumed preview, after its actual worker settled.
    Preview {
        /// Worktree storage namespace.
        worktree: WorktreeId,
        /// Capture storage namespace.
        capture: CaptureId,
        /// Non-reusable preview namespace.
        plan: vessel_proto::checkpoint::RestorePlanId,
    },
    /// An unpublished previous-process restore allocation, with no operation row.
    Restore {
        /// Worktree storage namespace.
        worktree: WorktreeId,
        /// Capture storage namespace.
        capture: CaptureId,
        /// Non-reusable restore namespace.
        restore: vessel_proto::checkpoint::RestoreId,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Accounted bytes cannot be released until this object's physical cleanup ends.
pub struct Garbage {
    /// Typed object identity; never a caller-supplied filesystem path.
    pub object: Object,
    /// Charge retained until verified physical cleanup.
    pub reserved: crate::ByteCount,
}
fn id<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, Error> {
    Ok(serde_json::from_str(&serde_json::to_string(raw)?)?)
}

/// Caller holds workspace exclusion and wraps this with session deletion in one savepoint.
pub(crate) fn retire_session(
    connection: &Connection,
    session: SessionId,
    at: DateTime<Utc>,
) -> Result<(), Error> {
    let session = session.to_string();
    let present: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM checkpoint_captures WHERE session=?1)",
        [&session],
        |row| row.get(0),
    )?;
    if !present {
        return Ok(());
    }
    let recovering:bool=connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM checkpoint_restores WHERE session=?1 AND phase IN ('prepared','recovery'))",
        [&session],|row|row.get(0),
    )?;
    if recovering || awaiting_metadata(connection, &session)? {
        return Err(CheckpointError::RecoveryPending.into());
    }
    let pinned:bool=connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM checkpoint_plans p JOIN checkpoint_captures c ON c.id=p.capture WHERE c.session=?1 AND p.expires_at>?2 AND p.consumed_by IS NULL)
         OR EXISTS(SELECT 1 FROM checkpoint_captures WHERE session=?1 AND (state='candidate' OR worker!='settled'))
         OR EXISTS(SELECT 1 FROM checkpoint_allocations a JOIN checkpoint_captures c ON c.id=a.capture WHERE c.session=?1 AND a.phase IN ('reserved','orphaned'))
         OR EXISTS(SELECT 1 FROM checkpoint_garbage g JOIN checkpoint_captures c ON json_extract(g.object,'$.capture')=c.id WHERE c.session=?1 AND json_extract(g.object,'$.kind') IN ('preview','restore'))",
        params![session,at.timestamp_millis()],|row|row.get(0),
    )?;
    if pinned {
        return Err(CheckpointError::Busy.into());
    }
    let mut statement = connection.prepare(
        "SELECT id,worktree,reserved_bytes + COALESCE((SELECT sum(a.reserved_bytes) FROM checkpoint_allocations a WHERE a.capture=checkpoint_captures.id),0) FROM checkpoint_captures WHERE session=?1 ORDER BY id",
    )?;
    let mut rows = statement.query([&session])?;
    while let Some(row) = rows.next()? {
        let capture: String = row.get(0)?;
        let worktree: String = row.get(1)?;
        let reserved: i64 = row.get(2)?;
        let object = Object::Capture {
            worktree: id(&worktree)?,
            capture: id(&capture)?,
        };
        let _changed = connection.execute(
            "INSERT INTO checkpoint_garbage(object,reserved_bytes,created_at) VALUES (?1,?2,?3) ON CONFLICT(object) DO UPDATE SET reserved_bytes=reserved_bytes+excluded.reserved_bytes",
            params![
                serde_json::to_string(&object)?,
                reserved,
                at.timestamp_millis()
            ],
        )?;
    }
    drop(rows);
    drop(statement);
    let _changed = connection.execute("DELETE FROM checkpoint_allocations WHERE capture IN (SELECT id FROM checkpoint_captures WHERE session=?1)", [&session])?;
    let _changed=connection.execute("DELETE FROM checkpoint_paths WHERE restore IN (SELECT id FROM checkpoint_restores WHERE session=?1)",[&session])?;
    let _changed = connection.execute(
        "DELETE FROM checkpoint_restores WHERE session=?1",
        [&session],
    )?;
    let _changed=connection.execute("DELETE FROM checkpoint_plans WHERE capture IN (SELECT id FROM checkpoint_captures WHERE session=?1)",[&session])?;
    let _changed = connection.execute(
        "DELETE FROM checkpoint_captures WHERE session=?1",
        [&session],
    )?;
    Ok(())
}

pub(crate) fn pending(connection: &Connection) -> Result<Vec<Garbage>, Error> {
    let mut statement = connection.prepare(
        "SELECT object,reserved_bytes FROM checkpoint_garbage ORDER BY created_at,object LIMIT ?1",
    )?;
    let limit =
        i64::try_from(CHECKPOINT_PAGE_ENTRIES).map_err(|_large| CheckpointError::TooManyPaths)?;
    let rows = statement
        .query_map([limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(object, reserved)| {
            Ok(Garbage {
                object: serde_json::from_str(&object)?,
                reserved: crate::ByteCount::new(
                    u64::try_from(reserved).map_err(|_negative| CheckpointError::StorageFull)?,
                ),
            })
        })
        .collect()
}

/// Only after all open worker handles and private files have actually been cleaned.
pub(crate) fn cleaned(connection: &Connection, object: Object) -> Result<(), Error> {
    let _changed = connection.execute(
        "DELETE FROM checkpoint_garbage WHERE object=?1",
        [serde_json::to_string(&object)?],
    )?;
    Ok(())
}

// A successful restore is not complete to clients until its metadata marker is
// durable. Even an expired preview must not let retention erase that obligation.
fn awaiting_metadata(connection: &Connection, session: &str) -> Result<bool, Error> {
    let mut statement = connection.prepare(
        "SELECT outcome FROM checkpoint_restores WHERE session=?1 AND phase='complete' AND event_seq IS NULL",
    )?;
    let mut rows = statement.query([session])?;
    while let Some(row) = rows.next()? {
        let outcome: String = row.get(0)?;
        if matches!(
            serde_json::from_str::<vessel_proto::checkpoint::RestoreOutcome>(&outcome)?,
            vessel_proto::checkpoint::RestoreOutcome::Restored { .. }
        ) {
            return Ok(true);
        }
    }
    Ok(false)
}
