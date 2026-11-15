//! Restart never treats an interrupted filesystem operation as committed success.
use super::records::Error;
use rusqlite::{Connection, params};
use vessel_proto::checkpoint::{CHECKPOINT_PAGE_ENTRIES, RestoreId, RestoreOutcome};

/// Advance a bounded batch; caller repeats while it receives a full batch.
pub(super) fn pending(connection: &Connection) -> Result<usize, Error> {
    let mut rows = connection.prepare(
        "SELECT id FROM checkpoint_restores WHERE phase='prepared' ORDER BY id LIMIT ?1",
    )?;
    let ids = rows
        .query_map(
            [i64::try_from(CHECKPOINT_PAGE_ENTRIES)
                .map_err(|_large| vessel_proto::checkpoint::CheckpointError::TooManyPaths)?],
            |row| row.get::<_, String>(0),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    for raw in &ids {
        let restore = raw.parse::<RestoreId>().map_err(Error::Id)?;
        let result = serde_json::to_string(&RestoreOutcome::RecoveryRequired { restore })?;
        connection.execute("UPDATE checkpoint_restores SET phase='recovery',outcome=?1 WHERE id=?2 AND phase='prepared'",params![result,raw])?;
    }
    Ok(ids.len())
}
