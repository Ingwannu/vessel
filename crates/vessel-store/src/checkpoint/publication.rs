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
         AND generation=?5 AND boundary=?6 AND state='candidate' AND image IS NOT NULL",
        params![checkpoint.to_string(),key.capture.to_string(),key.session.to_string(),key.worktree.to_string(),generation,boundary],
    )?;
    if changed != 1 {
        return Err(CheckpointError::StaleCapture.into());
    }
    Ok(())
}
