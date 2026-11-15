//! Only a live coordinator's accepted candidate may enter the Ready state.
//!
//! Candidate persistence does not imply activation. Startup invalidation and
//! expiration both change the row out of candidate; the conditional update
//! makes a late activation fail without reviving either boundary.
use super::records::CheckpointStoreError as Error;
use rusqlite::{Connection, params};
use vessel_proto::checkpoint::{CaptureKey, CheckpointError, CheckpointId};

pub(super) fn activate(
    connection: &Connection,
    key: CaptureKey,
    checkpoint: CheckpointId,
) -> Result<(), Error> {
    let generation = serde_json::to_string(&key.generation)?;
    let boundary = serde_json::to_string(&key.boundary)?;
    let changed = connection.execute(
        "UPDATE checkpoint_captures SET state='ready',checkpoint=?1 WHERE id=?2 AND session=?3 AND worktree=?4
         AND generation=?5 AND boundary=?6 AND state='candidate' AND image IS NOT NULL AND evidence IS NOT NULL",
        params![checkpoint.to_string(),key.capture.to_string(),key.session.to_string(),key.worktree.to_string(),generation,boundary],
    )?;
    if changed != 1 {
        return Err(CheckpointError::StaleCapture.into());
    }
    Ok(())
}

/// The immutable boundary must exist in SQL before capture admission or file I/O.
pub(super) fn boundary_persisted(connection: &Connection, key: CaptureKey) -> Result<(), Error> {
    use rusqlite::OptionalExtension as _;
    use vessel_proto::{
        AgentEvent, SessionEvent,
        checkpoint::{Boundary, CheckpointEvent},
    };
    let seq = match key.boundary {
        Boundary::BeforeTurn { prompt } => prompt,
        Boundary::AfterTurn { ended } => ended,
    };
    let seq = i64::try_from(seq.get()).map_err(|_large| CheckpointError::StaleCapture)?;
    let event: Option<String> = connection
        .query_row(
            "SELECT event FROM events WHERE session_id=?1 AND seq=?2",
            params![key.session.to_string(), seq],
            |row| row.get(0),
        )
        .optional()?;
    let event: SessionEvent = serde_json::from_str(&event.ok_or(CheckpointError::StaleCapture)?)?;
    let valid = match key.boundary {
        Boundary::BeforeTurn { .. } => matches!(
            event,
            SessionEvent::Checkpoint {
                event: CheckpointEvent::PromptAccepted
            }
        ),
        Boundary::AfterTurn { .. } => matches!(
            event,
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded { .. }
            }
        ),
    };
    if !valid {
        return Err(CheckpointError::StaleCapture.into());
    }
    Ok(())
}

/// Existing active/ready rows belong to their original worker and are unchanged.
/// The actor has already received its refusal; this bounded metadata ack cannot
/// prolong its capture deadline or create a ready image without filesystem work.
pub(super) fn refused(
    connection: &Connection,
    key: CaptureKey,
    reason: CheckpointError,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<(), Error> {
    use rusqlite::OptionalExtension as _;
    let generation = serde_json::to_string(&key.generation)?;
    let boundary = serde_json::to_string(&key.boundary)?;
    let present:Option<(String,String,String,String)>=connection.query_row("SELECT session,worktree,CAST(generation AS TEXT),boundary FROM checkpoint_captures WHERE id=?1",[key.capture.to_string()],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional()?;
    if let Some((session, worktree, stored_generation, stored_boundary)) = present {
        return if session == key.session.to_string()
            && worktree == key.worktree.to_string()
            && stored_generation == generation
            && stored_boundary == boundary
        {
            Ok(())
        } else {
            Err(CheckpointError::StaleCapture.into())
        };
    }
    boundary_persisted(connection, key)?;
    super::quota::available(
        connection,
        crate::ByteCount::new(super::quota::METADATA_BYTES),
    )?;
    let charge = i64::try_from(super::quota::METADATA_BYTES)
        .map_err(|_large| CheckpointError::StorageFull)?;
    let _inserted=connection.execute("INSERT INTO checkpoint_captures(id,session,worktree,generation,boundary,state,worker,reason,reserved_bytes,created_at) VALUES (?1,?2,?3,?4,?5,'unavailable','settled',?6,?7,?8)",params![key.capture.to_string(),key.session.to_string(),key.worktree.to_string(),generation,boundary,serde_json::to_string(&reason)?,charge,at.timestamp_millis()])?;
    Ok(())
}
