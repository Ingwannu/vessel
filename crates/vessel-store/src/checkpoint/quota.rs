//! One persistent accounting equation includes active, private and obsolete bytes.
use super::records::CheckpointStoreError as Error;
use rusqlite::Connection;
use vessel_proto::checkpoint::{CHECKPOINT_NODE_BYTES, CheckpointError};
pub(super) fn available(
    connection: &Connection,
    additional: crate::ByteCount,
) -> Result<(), Error> {
    let used: i64 = connection.query_row(
        "SELECT COALESCE((SELECT sum(reserved_bytes) FROM checkpoint_captures),0)
        + COALESCE((SELECT sum(reserved_bytes) FROM checkpoint_allocations),0)
        + COALESCE((SELECT sum(reserved_bytes) FROM checkpoint_garbage),0)",
        [],
        |row| row.get(0),
    )?;
    let used = u64::try_from(used).map_err(|_negative| CheckpointError::StorageFull)?;
    let total = used
        .checked_add(additional.get())
        .ok_or(CheckpointError::StorageFull)?;
    if total > CHECKPOINT_NODE_BYTES {
        return Err(CheckpointError::StorageFull.into());
    }
    Ok(())
}

/// Escrow before the first read: raw image, candidate, metadata and file overhead.
pub(crate) const CAPTURE_BYTES: crate::ByteCount =
    crate::ByteCount::new(vessel_proto::checkpoint::CHECKPOINT_CAPTURE_RESERVATION_BYTES);
const METADATA_BYTES: u64 = 64 * 1024;
pub(super) const MANIFEST_BYTES: u64 = 64 * 1024 * 1024;
const METADATA_COPIES: u64 = 3;
const ENTRY_ALLOWANCE: u64 = 64 * 1024;

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum State {
    Candidate,
    Ready,
    Unavailable,
}
fn decode_state(raw: &str) -> Result<State, Error> {
    Ok(serde_json::from_str(&serde_json::to_string(raw)?)?)
}

/// Only after physical worker/candidate cleanup. A ready archive stays charged;
/// an unavailable record retains metadata. It never grows an undersized escrow.
pub(crate) fn capture_settled(
    connection: &Connection,
    key: vessel_proto::checkpoint::CaptureKey,
) -> Result<(), Error> {
    use rusqlite::{OptionalExtension as _, params};
    let generation = serde_json::to_string(&key.generation)?;
    let boundary = serde_json::to_string(&key.boundary)?;
    let row: Option<(String,i64,i64)> = connection.query_row(
        "SELECT state,reserved_bytes,COALESCE(length(CAST(image AS BLOB)),0)+COALESCE(length(CAST(evidence AS BLOB)),0)
        FROM checkpoint_captures WHERE id=?1 AND session=?2 AND worktree=?3 AND generation=?4 AND boundary=?5",
        params![key.capture.to_string(),key.session.to_string(),key.worktree.to_string(),generation,boundary],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
    ).optional()?;
    let (state, reserved, metadata) = row.ok_or(CheckpointError::StaleCapture)?;
    let metadata = u64::try_from(metadata).map_err(|_negative| CheckpointError::StorageFull)?;
    if metadata > MANIFEST_BYTES {
        return Err(CheckpointError::StorageFull.into());
    }
    let charged = match decode_state(&state)? {
        State::Candidate => return Err(CheckpointError::Busy.into()),
        State::Unavailable => METADATA_BYTES,
        State::Ready => ready_charge(connection, key.capture, metadata)?,
    };
    let reserved = u64::try_from(reserved).map_err(|_negative| CheckpointError::StorageFull)?;
    if charged > reserved {
        return Err(CheckpointError::StorageFull.into());
    }
    let charged = i64::try_from(charged).map_err(|_large| CheckpointError::StorageFull)?;
    let changed = connection.execute("UPDATE checkpoint_captures SET reserved_bytes=?1 WHERE id=?2 AND generation=?3 AND boundary=?4 AND state=?5 AND reserved_bytes=?6",params![charged,key.capture.to_string(),generation,boundary,state,i64::try_from(reserved).map_err(|_large|CheckpointError::StorageFull)?])?;
    if changed != 1 {
        return Err(CheckpointError::StaleCapture.into());
    }
    Ok(())
}
fn ready_charge(
    connection: &Connection,
    capture: vessel_proto::checkpoint::CaptureId,
    metadata: u64,
) -> Result<u64, Error> {
    use vessel_proto::checkpoint::{FileState, ObservedSnapshot};
    let (image, evidence): (String, String) = connection.query_row(
        "SELECT image,evidence FROM checkpoint_captures WHERE id=?1",
        [capture.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let snapshot = ObservedSnapshot::new(
        serde_json::from_str(&image)?,
        serde_json::from_str(&evidence)?,
    )?;
    let mut bytes = metadata
        .checked_mul(METADATA_COPIES)
        .and_then(|bytes| bytes.checked_add(METADATA_BYTES))
        .ok_or(CheckpointError::StorageFull)?;
    for entry in snapshot.image().entries() {
        if let FileState::Present { bytes: payload, .. } = entry.state {
            bytes = bytes
                .checked_add(payload.get())
                .and_then(|bytes| bytes.checked_add(ENTRY_ALLOWANCE))
                .ok_or(CheckpointError::StorageFull)?;
        }
    }
    Ok(bytes)
}
