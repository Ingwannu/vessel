//! Completed private payloads can retire without forgetting idempotency records.
use super::{Allocation, GarbageObject, records::CheckpointStoreError as Error};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::checkpoint::{CHECKPOINT_PAGE_ENTRIES, CHECKPOINT_PATHS, CheckpointError};
const ROW_ALLOWANCE: u64 = 64 * 1024;
const METADATA_COPIES: u64 = 3;

pub(super) fn collect(connection: &Connection, at: DateTime<Utc>) -> Result<(), Error> {
    let mut statement=connection.prepare("SELECT a.object,a.capture,c.worktree,a.reserved_bytes FROM checkpoint_allocations a JOIN checkpoint_captures c ON c.id=a.capture WHERE a.phase='attached' AND c.worker='settled' AND EXISTS(SELECT 1 FROM checkpoint_restores r JOIN checkpoint_plans p ON p.id=r.plan WHERE p.capture=c.id AND r.phase='complete' AND (json_extract(r.outcome,'$.kind')!='restored' OR r.event_seq IS NOT NULL) AND ((json_extract(a.object,'$.kind')='preview' AND json_extract(a.object,'$.plan')=p.id) OR (json_extract(a.object,'$.kind')='restore' AND json_extract(a.object,'$.restore')=r.id))) ORDER BY a.object LIMIT ?1")?;
    let limit =
        i64::try_from(CHECKPOINT_PAGE_ENTRIES).map_err(|_large| CheckpointError::TooManyPaths)?;
    let rows = statement
        .query_map([limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    for (object, capture, worktree, reserved) in rows {
        let allocation = serde_json::from_str::<Allocation>(&object)?;
        let retained = metadata_charge(connection, allocation)?;
        let retired = reserved
            .checked_sub(retained)
            .filter(|bytes| *bytes >= 0)
            .ok_or(CheckpointError::StorageFull)?;
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
        let _charge=connection.execute("UPDATE checkpoint_allocations SET phase='cleaned',reserved_bytes=?1 WHERE object=?2 AND phase='attached'",params![retained,object])?;
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
fn metadata_charge(connection: &Connection, allocation: Allocation) -> Result<i64, Error> {
    let (bytes,rows):(i64,i64)=match allocation {
        Allocation::Preview {plan} => connection.query_row("SELECT length(CAST(current_image AS BLOB))+length(CAST(current_evidence AS BLOB))+length(CAST(desired_image AS BLOB)),1 FROM checkpoint_plans WHERE id=?1",[plan.to_string()],|row|Ok((row.get(0)?,row.get(1)?)))?,
        Allocation::Restore {restore} => connection.query_row("SELECT length(CAST(before_image AS BLOB))+length(CAST(outcome AS BLOB))+COALESCE((SELECT sum(length(CAST(path AS BLOB))+length(CAST(before_state AS BLOB))+length(CAST(desired_state AS BLOB))+COALESCE(length(CAST(staged AS BLOB)),0)+length(phase)) FROM checkpoint_paths WHERE restore=r.id),0),(SELECT count(*) FROM checkpoint_paths WHERE restore=r.id)+1 FROM checkpoint_restores r WHERE id=?1",[restore.to_string()],|row|Ok((row.get(0)?,row.get(1)?)))?,
    };
    let bytes = u64::try_from(bytes).map_err(|_negative| CheckpointError::StorageFull)?;
    let rows = u64::try_from(rows).map_err(|_negative| CheckpointError::StorageFull)?;
    if bytes > super::quota::MANIFEST_BYTES
        || rows
            > u64::try_from(CHECKPOINT_PATHS.saturating_add(1))
                .map_err(|_large| CheckpointError::TooManyPaths)?
    {
        return Err(CheckpointError::StorageFull.into());
    }
    let charged = bytes
        .checked_mul(METADATA_COPIES)
        .and_then(|bytes| {
            rows.checked_mul(ROW_ALLOWANCE)
                .and_then(|overhead| bytes.checked_add(overhead))
        })
        .ok_or(CheckpointError::StorageFull)?;
    Ok(i64::try_from(charged).map_err(|_large| CheckpointError::StorageFull)?)
}
