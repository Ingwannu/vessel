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
