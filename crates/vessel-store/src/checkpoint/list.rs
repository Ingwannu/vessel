//! Session-bound keyset pagination never accepts a cursor from another session.
use super::records::CheckpointStoreError as Error;
use rusqlite::{Connection, OptionalExtension as _, params};
use serde::Deserialize;
use vessel_proto::{
    SessionId,
    checkpoint::{
        Boundary, CHECKPOINT_PAGE_ENTRIES, CaptureGeneration, CaptureId, CheckpointCursor,
        CheckpointError, CheckpointId, CheckpointMark, CheckpointPage, CheckpointState,
    },
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum State {
    Candidate,
    Ready,
    Unavailable,
    Expired,
}
struct Stored {
    capture: String,
    generation: i64,
    boundary: String,
    state: String,
    checkpoint: Option<String>,
    reason: Option<String>,
}
fn decode<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, Error> {
    Ok(serde_json::from_str(&serde_json::to_string(text)?)?)
}
fn mark(raw: &Stored) -> Result<CheckpointMark, Error> {
    let state = match decode::<State>(&raw.state)? {
        State::Expired => CheckpointState::Expired,
        State::Candidate => CheckpointState::Unavailable {
            reason: CheckpointError::Busy,
        },
        State::Ready => CheckpointState::Ready {
            checkpoint: decode::<CheckpointId>(
                raw.checkpoint.as_deref().ok_or(CheckpointError::NotFound)?,
            )?,
        },
        State::Unavailable => CheckpointState::Unavailable {
            reason: serde_json::from_str(raw.reason.as_deref().ok_or(CheckpointError::NotFound)?)?,
        },
    };
    Ok(CheckpointMark {
        boundary: serde_json::from_str::<Boundary>(&raw.boundary)?,
        generation: CaptureGeneration::try_from(
            u64::try_from(raw.generation).map_err(|_negative| CheckpointError::StaleCapture)?,
        )?,
        state,
    })
}

pub(super) fn page(
    connection: &Connection,
    session: SessionId,
    after: Option<CheckpointCursor>,
) -> Result<CheckpointPage, Error> {
    let anchor = after
        .map(|cursor| {
            connection
                .query_row(
                    "SELECT created_at,id FROM checkpoint_captures WHERE session=?1 AND id=?2",
                    params![session.to_string(), cursor.0.to_string()],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?
                .ok_or_else(|| Error::from(CheckpointError::InvalidCursor))
        })
        .transpose()?;
    let (time, id) = match &anchor {
        Some((time, id)) => (Some(*time), Some(id.as_str())),
        None => (None, None),
    };
    let limit = i64::try_from(CHECKPOINT_PAGE_ENTRIES.saturating_add(1))
        .map_err(|_large| CheckpointError::TooManyPaths)?;
    let mut statement=connection.prepare(
        "SELECT id,generation,boundary,state,checkpoint,reason FROM checkpoint_captures WHERE session=?1
        AND (?2 IS NULL OR created_at<?2 OR (created_at=?2 AND id<?3)) ORDER BY created_at DESC,id DESC LIMIT ?4")?;
    let mut rows = statement
        .query_map(params![session.to_string(), time, id, limit], |row| {
            Ok(Stored {
                capture: row.get(0)?,
                generation: row.get(1)?,
                boundary: row.get(2)?,
                state: row.get(3)?,
                checkpoint: row.get(4)?,
                reason: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let more = rows.len() > CHECKPOINT_PAGE_ENTRIES;
    rows.truncate(CHECKPOINT_PAGE_ENTRIES);
    let next = if more {
        rows.last()
            .map(|row| decode::<CaptureId>(&row.capture).map(CheckpointCursor))
            .transpose()?
    } else {
        None
    };
    Ok(CheckpointPage {
        marks: rows.iter().map(mark).collect::<Result<_, _>>()?,
        next,
    })
}
#[cfg(test)]
mod tests;
