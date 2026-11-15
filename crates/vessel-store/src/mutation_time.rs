//! The single writer-owned session mutation clock, on the caller's connection.
//! Never use record/client time or advance this for reads and maintenance.

use crate::{
    StoreError,
    codec::{time_from_sql, time_to_sql},
};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::{SessionId, SessionUpdatedAt};

pub(crate) fn read(
    connection: &Connection,
    session: SessionId,
) -> Result<Option<SessionUpdatedAt>, StoreError> {
    let stored: Option<Option<i64>> = connection
        .prepare_cached("SELECT updated_at FROM sessions WHERE id=?1")?
        .query_row([session.to_string()], |row| row.get(0))
        .optional()?;
    stored
        .ok_or(StoreError::UnknownSession(session))?
        .map(|nanos| {
            SessionUpdatedAt::try_from(time_from_sql(nanos))
                .map_err(|source| StoreError::MutationTime { session, source })
        })
        .transpose()
}

/// The caller's existing savepoint owns the mutation and this tick together.
/// Returning a value here is not a commit acknowledgement; only the writer's
/// normal reply, after the outer transaction commits, provides that boundary.
pub(crate) fn advance(
    connection: &Connection,
    session: SessionId,
) -> Result<SessionUpdatedAt, StoreError> {
    let previous = read(connection, session)?;
    let updated = SessionUpdatedAt::advance(previous, Utc::now())
        .map_err(|source| StoreError::MutationTime { session, source })?;
    let _rows = connection.execute(
        "UPDATE sessions SET updated_at=?2 WHERE id=?1",
        params![session.to_string(), time_to_sql(updated.time())?],
    )?;
    Ok(updated)
}
