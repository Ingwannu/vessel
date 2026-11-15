//! Retire oldest unpinned payloads while preserving boundary/list cursor metadata.
//!
//! [Decision Log]
//! - Purpose: the per-session image limit must not free bytes ahead of actual GC.
//! - Constraint: active workers, preview leases and restore journals own payloads.
//! - Choice: one bounded writer transaction transfers obsolete bytes to durable
//!   garbage and leaves Expired boundary/id tombstones. Pinned images are extra.
//! - Cost: GC failure retains the full charge; completed result metadata survives.
use super::{GarbageObject, records::CheckpointStoreError as Error};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::checkpoint::{CHECKPOINT_PAGE_ENTRIES, CHECKPOINTS_PER_SESSION, CheckpointError};
const TOMBSTONE_BYTES: i64 = 64 * 1024;
const ELIGIBLE: &str = "state='ready' AND worker='settled'
 AND NOT EXISTS(SELECT 1 FROM checkpoint_plans p WHERE p.capture=c.id AND p.expires_at>?1 AND p.consumed_by IS NULL)
 AND NOT EXISTS(SELECT 1 FROM checkpoint_restores r JOIN checkpoint_plans p ON p.id=r.plan WHERE p.capture=c.id AND (r.phase!='complete' OR (json_extract(r.outcome,'$.kind')='restored' AND r.event_seq IS NULL)))
 AND NOT EXISTS(SELECT 1 FROM checkpoint_allocations a WHERE a.capture=c.id AND a.phase='attached' AND EXISTS(SELECT 1 FROM checkpoint_restores r JOIN checkpoint_plans p ON p.id=r.plan WHERE p.capture=c.id))
 AND NOT EXISTS(SELECT 1 FROM checkpoint_allocations a WHERE a.capture=c.id AND a.phase IN ('reserved','orphaned'))
 AND NOT EXISTS(SELECT 1 FROM checkpoint_garbage g WHERE json_extract(g.object,'$.capture')=c.id)";

pub(super) fn collect(connection: &Connection, at: DateTime<Utc>) -> Result<(), Error> {
    super::private_orphans::collect(connection, at)?;
    super::completed::collect(connection, at)?;
    expire_previews(connection, at)?;
    let query = format!("WITH eligible AS (SELECT id,worktree,session,created_at FROM checkpoint_captures c WHERE {ELIGIBLE}), ranked AS (
        SELECT id,worktree,ROW_NUMBER() OVER (PARTITION BY session ORDER BY created_at DESC,id DESC) AS position FROM eligible)
        SELECT id,worktree FROM ranked WHERE position>?2 ORDER BY id LIMIT ?3");
    let mut statement = connection.prepare(&query)?;
    let keep =
        i64::try_from(CHECKPOINTS_PER_SESSION).map_err(|_large| CheckpointError::TooManyPaths)?;
    let limit =
        i64::try_from(CHECKPOINT_PAGE_ENTRIES).map_err(|_large| CheckpointError::TooManyPaths)?;
    let obsolete = statement
        .query_map(params![at.timestamp_millis(), keep, limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    for (capture, worktree) in obsolete {
        let _retired = retire(connection, &capture, &worktree, at)?;
    }
    pressure(connection, at)
}
fn retire(
    connection: &Connection,
    capture: &str,
    worktree: &str,
    at: DateTime<Utc>,
) -> Result<crate::ByteCount, Error> {
    let total: i64 = connection.query_row("SELECT reserved_bytes+COALESCE((SELECT sum(reserved_bytes) FROM checkpoint_allocations WHERE capture=?1),0) FROM checkpoint_captures WHERE id=?1", [capture], |row| row.get(0))?;
    // Eligible captures have no consuming restore, so unconsumed expired plans have no remaining file authority. Completed plan/
    // result metadata and allocation ids remain for truthful idempotent retries.
    let _plans = connection.execute(
        "DELETE FROM checkpoint_plans WHERE capture=?1 AND consumed_by IS NULL",
        [capture],
    )?;
    let _allocations = connection.execute("UPDATE checkpoint_allocations SET phase='cleaned',reserved_bytes=CASE WHEN phase='cleaned' THEN reserved_bytes ELSE MIN(reserved_bytes,?2) END WHERE capture=?1", params![capture,TOMBSTONE_BYTES])?;
    let kept: i64 = connection.query_row("SELECT MIN(reserved_bytes,?2)+COALESCE((SELECT sum(reserved_bytes) FROM checkpoint_allocations WHERE capture=?1),0) FROM checkpoint_captures WHERE id=?1", params![capture,TOMBSTONE_BYTES],|row|row.get(0))?;
    let garbage = total
        .checked_sub(kept)
        .ok_or(CheckpointError::StorageFull)?;
    let object = GarbageObject::Capture {
        worktree: super::query::text(worktree)?,
        capture: super::query::text(capture)?,
    };
    let _garbage = connection.execute(
        "INSERT INTO checkpoint_garbage(object,reserved_bytes,created_at) VALUES (?1,?2,?3)",
        params![
            serde_json::to_string(&object)?,
            garbage,
            at.timestamp_millis()
        ],
    )?;
    let _expired = connection.execute("UPDATE checkpoint_captures SET state='expired',image=NULL,evidence=NULL,reserved_bytes=MIN(reserved_bytes,?2) WHERE id=?1 AND state='ready' AND worker='settled'",params![capture,TOMBSTONE_BYTES])?;
    Ok(crate::ByteCount::new(
        u64::try_from(garbage).map_err(|_negative| CheckpointError::StorageFull)?,
    ))
}

/// Only Attached is eligible: an expiry does not certify an active worker ended.
fn expire_previews(connection: &Connection, at: DateTime<Utc>) -> Result<(), Error> {
    let mut statement = connection.prepare("SELECT p.id,p.capture,c.worktree,a.reserved_bytes FROM checkpoint_plans p JOIN checkpoint_captures c ON c.id=p.capture JOIN checkpoint_allocations a ON a.capture=c.id AND json_extract(a.object,'$.kind')='preview' AND json_extract(a.object,'$.plan')=p.id WHERE p.expires_at<=?1 AND p.consumed_by IS NULL AND a.phase='attached' ORDER BY p.id LIMIT ?2")?;
    let limit =
        i64::try_from(CHECKPOINT_PAGE_ENTRIES).map_err(|_large| CheckpointError::TooManyPaths)?;
    let obsolete = statement
        .query_map(params![at.timestamp_millis(), limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    for (plan, capture, worktree, reserved) in obsolete {
        let plan = super::query::text(&plan)?;
        let allocation = super::Allocation::Preview { plan };
        let object = GarbageObject::Preview {
            worktree: super::query::text(&worktree)?,
            capture: super::query::text(&capture)?,
            plan,
        };
        let kept = reserved.min(TOMBSTONE_BYTES);
        let garbage = reserved
            .checked_sub(kept)
            .ok_or(CheckpointError::StorageFull)?;
        let _removed = connection.execute(
            "DELETE FROM checkpoint_plans WHERE id=?1 AND consumed_by IS NULL",
            [plan.to_string()],
        )?;
        let _charge = connection.execute("UPDATE checkpoint_allocations SET phase='cleaned',reserved_bytes=?1 WHERE object=?2 AND phase='attached'",params![kept,serde_json::to_string(&allocation)?])?;
        let _garbage = connection.execute(
            "INSERT INTO checkpoint_garbage(object,reserved_bytes,created_at) VALUES (?1,?2,?3)",
            params![
                serde_json::to_string(&object)?,
                garbage,
                at.timestamp_millis()
            ],
        )?;
    }
    Ok(())
}

/// Plan enough physical collection to admit the fixed capture envelope. Pending
/// garbage already promises that reclamation, but remains charged until I/O ends.
fn pressure(connection: &Connection, at: DateTime<Utc>) -> Result<(), Error> {
    use vessel_proto::checkpoint::{CHECKPOINT_CAPTURE_RESERVATION_BYTES, CHECKPOINT_NODE_BYTES};
    let used = super::quota::used(connection)?.get();
    let garbage: i64 = connection.query_row(
        "SELECT COALESCE(sum(reserved_bytes),0) FROM checkpoint_garbage",
        [],
        |row| row.get(0),
    )?;
    let live = used
        .checked_sub(u64::try_from(garbage).map_err(|_negative| CheckpointError::StorageFull)?)
        .ok_or(CheckpointError::StorageFull)?;
    let mut needed = live
        .checked_add(CHECKPOINT_CAPTURE_RESERVATION_BYTES)
        .ok_or(CheckpointError::StorageFull)?
        .saturating_sub(CHECKPOINT_NODE_BYTES);
    if needed == 0 {
        return Ok(());
    }
    let mut statement=connection.prepare(&format!("SELECT id,worktree FROM checkpoint_captures c WHERE {ELIGIBLE} ORDER BY created_at,id LIMIT ?2"))?;
    let limit =
        i64::try_from(CHECKPOINT_PAGE_ENTRIES).map_err(|_large| CheckpointError::TooManyPaths)?;
    let rows = statement
        .query_map(params![at.timestamp_millis(), limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    for (capture, worktree) in rows {
        if needed == 0 {
            break;
        }
        needed = needed.saturating_sub(retire(connection, &capture, &worktree, at)?.get());
    }
    Ok(())
}
