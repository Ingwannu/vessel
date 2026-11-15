//! Only startup-fenced private workers can enter this bounded orphan collector.
use super::{Allocation, GarbageObject, records::CheckpointStoreError as Error};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::checkpoint::{CHECKPOINT_PAGE_ENTRIES, CheckpointError};
const TOMBSTONE_BYTES: i64 = 64 * 1024;

/// The held node data lock proves previous-process private I/O ended. Referenced
/// objects remain fully charged/pinned; this does not verify their file payloads.
/// The restore worker verifies referenced archives again before first mutation.
pub(super) fn collect(connection: &Connection, at: DateTime<Utc>) -> Result<(), Error> {
    let mut statement = connection.prepare("SELECT a.object,a.capture,c.worktree,a.reserved_bytes,a.owner FROM checkpoint_allocations a JOIN checkpoint_captures c ON c.id=a.capture WHERE a.phase='orphaned' ORDER BY a.object LIMIT ?1")?;
    let limit =
        i64::try_from(CHECKPOINT_PAGE_ENTRIES).map_err(|_large| CheckpointError::TooManyPaths)?;
    let rows = statement
        .query_map([limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    for (object, capture, worktree, reserved, owner) in rows {
        let allocation = serde_json::from_str::<Allocation>(&object)?;
        let referenced = match allocation {
            Allocation::Preview { plan } => connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM checkpoint_plans WHERE id=?1 AND capture=?2 AND owner=?3)",
                params![plan.to_string(), capture, owner],
                |row| row.get::<_, bool>(0),
            )?,
            Allocation::Restore { restore } => connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM checkpoint_restores r JOIN checkpoint_plans p ON p.id=r.plan WHERE r.id=?1 AND p.capture=?2 AND r.owner=?3)",
                params![restore.to_string(), capture, owner],
                |row| row.get::<_, bool>(0),
            )?,
        };
        if referenced {
            let _settled = connection.execute("UPDATE checkpoint_allocations SET phase='attached' WHERE object=?1 AND phase='orphaned'",[object])?;
            continue;
        }
        let capture = super::query::text(&capture)?;
        let worktree = super::query::text(&worktree)?;
        let garbage = match allocation {
            Allocation::Preview { plan } => GarbageObject::Preview {
                worktree,
                capture,
                plan,
            },
            Allocation::Restore { restore } => GarbageObject::Restore {
                worktree,
                capture,
                restore,
            },
        };
        let kept = reserved.min(TOMBSTONE_BYTES);
        let retired = reserved
            .checked_sub(kept)
            .ok_or(CheckpointError::StorageFull)?;
        let _charge = connection.execute("UPDATE checkpoint_allocations SET phase='cleaned',reserved_bytes=?1 WHERE object=?2 AND phase='orphaned'",params![kept,object])?;
        let _garbage = connection.execute(
            "INSERT INTO checkpoint_garbage(object,reserved_bytes,created_at) VALUES (?1,?2,?3)",
            params![
                serde_json::to_string(&garbage)?,
                retired,
                at.timestamp_millis()
            ],
        )?;
    }
    Ok(())
}
