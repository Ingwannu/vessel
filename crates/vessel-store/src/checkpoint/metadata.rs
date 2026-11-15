//! Files-restored metadata shares the log transaction and durable restore row.
//!
//! An ended run has no actor to allocate sequence numbers. The existing writer
//! allocates one while checking `live`; a resumed actor therefore cannot race
//! this path. Live actors use ordinary append and the same projection below.
use super::{journal, records::Operation};
use crate::{StoreError, codec, log};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::{
    EventRecord, EventSeq, SessionEvent, SessionId,
    checkpoint::{CheckpointError, CheckpointEvent, CheckpointId, RestoreId, RestoreOutcome},
};

struct Binding {
    event: CheckpointEvent,
    seq: Option<EventSeq>,
}

fn refused(reason: CheckpointError) -> StoreError {
    super::CheckpointStoreError::Refused(reason).into()
}

fn binding(
    connection: &Connection,
    session: SessionId,
    restore: RestoreId,
) -> Result<Binding, StoreError> {
    let raw: Option<(String, String, Option<i64>)> = connection
        .query_row(
            "SELECT r.outcome,c.checkpoint,r.event_seq FROM checkpoint_restores r
         JOIN checkpoint_plans p ON p.id=r.plan JOIN checkpoint_captures c ON c.id=p.capture
         WHERE r.id=?1 AND r.session=?2 AND r.phase='complete' AND c.state='ready'",
            params![restore.to_string(), session.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (outcome, checkpoint, seq) = raw.ok_or_else(|| refused(CheckpointError::NotFound))?;
    let outcome: RestoreOutcome =
        serde_json::from_str(&outcome).map_err(super::CheckpointStoreError::from)?;
    let RestoreOutcome::Restored { changed } = outcome else {
        return Err(refused(CheckpointError::RestoreIdConflict));
    };
    let checkpoint: CheckpointId = serde_json::from_str(&codec::encode(&checkpoint)?)
        .map_err(super::CheckpointStoreError::from)?;
    Ok(Binding {
        event: CheckpointEvent::FilesRestored {
            restore,
            checkpoint,
            paths: changed,
        },
        seq: seq.map(codec::seq_from_sql).transpose()?,
    })
}

/// Called by log append inside its savepoint: event and deduplication commit together.
pub(crate) fn observe(
    connection: &Connection,
    session: SessionId,
    record: &EventRecord,
) -> Result<(), StoreError> {
    let SessionEvent::Checkpoint {
        event: CheckpointEvent::FilesRestored { restore, .. },
    } = &record.event
    else {
        return Ok(());
    };
    let selected = binding(connection, session, *restore)?;
    if record.event
        != (SessionEvent::Checkpoint {
            event: selected.event,
        })
        || selected.seq.is_some()
    {
        return Err(refused(CheckpointError::RestoreIdConflict));
    }
    let changed = connection.execute(
        "UPDATE checkpoint_restores SET event_seq=?1 WHERE id=?2 AND session=?3 AND event_seq IS NULL",
        params![codec::seq_to_sql(record.seq)?, restore.to_string(), session.to_string()],
    )?;
    if changed != 1 {
        return Err(refused(CheckpointError::RestoreIdConflict));
    }
    Ok(())
}

pub(super) fn append_ended(
    connection: &Connection,
    operation: &Operation,
    at: DateTime<Utc>,
) -> Result<(), StoreError> {
    let current = journal::status(connection, operation.session, operation.id, operation.owner)?;
    if current.plan != operation.plan {
        return Err(refused(CheckpointError::RestoreIdConflict));
    }
    let selected = binding(connection, current.session, current.id)?;
    if selected.seq.is_some() {
        return Ok(());
    }
    let (live, next): (bool, i64) = connection.query_row(
        "SELECT live,next_seq FROM sessions WHERE id=?1",
        [current.session.to_string()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if live {
        return Err(refused(CheckpointError::Busy));
    }
    let record = EventRecord {
        seq: codec::seq_from_sql(next)?,
        at,
        event: SessionEvent::Checkpoint {
            event: selected.event,
        },
    };
    let _appended = log::append(connection, current.session, [&record])?;
    Ok(())
}
