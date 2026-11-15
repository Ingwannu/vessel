//! Startup intent and internal session checkpoints; never a second event log.
mod state;
use crate::codec::{decode, encode, session_id_from_sql, time_to_sql};
use crate::error::{Column, StoreError};
use crate::reader::select_session;
use rusqlite::{Connection, OptionalExtension as _, params};
pub use state::Eligibility;
pub(crate) use state::State;
use vessel_proto::restart::{RestartCheckpoint, RestartGeneration};
use vessel_proto::{Creator, SessionId, SessionMode, SessionOrigin, SessionStatus};

/// An indexed, passive startup candidate; claiming it still validates current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestartCandidate {
    /// Same ordinary session, not a replacement conversation.
    pub session: SessionId,
    /// Stop/admission generation observed by this read.
    pub generation: RestartGeneration,
    /// The interrupted turn in that session's own log.
    pub turn: vessel_proto::EventSeq,
}
/// Changes made only by the node's single writer.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RestartWrite {
    Configure {
        session: SessionId,
        eligibility: Eligibility,
    },
    Checkpoint {
        session: SessionId,
        change: RestartCheckpoint,
    },
    Policy {
        enabled: bool,
    },
    Claim(RestartCandidate),
    SuppressEnded {
        session: SessionId,
        next: vessel_proto::EventSeq,
    },
    Finish {
        session: SessionId,
    },
}
pub(crate) fn load(
    connection: &Connection,
    session: SessionId,
) -> Result<Option<State>, StoreError> {
    let text: Option<String> = connection
        .prepare_cached("SELECT state FROM restart_state WHERE session_id = ?1")?
        .query_row([session.to_string()], |row| row.get(0))
        .optional()?;
    text.map(|text| decode(session, Column::Restart, &text))
        .transpose()
}
pub(crate) fn save(
    connection: &Connection,
    session: SessionId,
    state: &State,
) -> Result<(), StoreError> {
    let active = state
        .active
        .filter(|_| state.eligible() && !state.blocked && !state.parked);
    let _changed = connection
        .prepare_cached(
            "INSERT INTO restart_state(session_id,state,active_at,pending_at,claimed)
        VALUES(?1,?2,?3,?4,?5) ON CONFLICT(session_id) DO UPDATE SET state=excluded.state,
        active_at=excluded.active_at,pending_at=excluded.pending_at,claimed=excluded.claimed",
        )?
        .execute(params![
            session.to_string(),
            encode(state)?,
            active.map(|turn| time_to_sql(turn.at)).transpose()?,
            state.pending.map(|turn| time_to_sql(turn.at)).transpose()?,
            state.claimed
        ])?;
    Ok(())
}
fn policy(connection: &Connection) -> Result<bool, StoreError> {
    Ok(connection.query_row(
        "SELECT enabled FROM restart_control WHERE singleton=1",
        [],
        |row| row.get(0),
    )?)
}
fn eligible(connection: &Connection, session: SessionId) -> Result<bool, StoreError> {
    Ok(select_session(connection, session)?.is_some_and(|info| {
        matches!(info.spec.mode, SessionMode::Structured { .. })
            && info.spec.side_question_of.is_none()
            && matches!(info.created_by, Creator::Owner | Creator::Device { .. })
            && matches!(info.origin, None | Some(SessionOrigin::Imported { .. }))
            && info.marks.archived_at.is_none()
    }))
}
/// One writer operation, whose boolean distinguishes a stale claim/checkpoint.
pub(crate) fn write(connection: &Connection, change: RestartWrite) -> Result<bool, StoreError> {
    match change {
        RestartWrite::Configure {
            session,
            eligibility,
        } => {
            if select_session(connection, session)?.is_none() {
                return Ok(false);
            }
            let mut state = load(connection, session)?.unwrap_or_default();
            state.eligibility =
                if eligibility == Eligibility::Person && eligible(connection, session)? {
                    Eligibility::Person
                } else {
                    Eligibility::Never
                };
            if !state.eligible() {
                state.pending = None;
            }
            save(connection, session, &state)?;
        }
        RestartWrite::Checkpoint { session, change } => {
            let Some(mut state) = load(connection, session)? else {
                return Ok(false);
            };
            if !state.checkpoint(change, policy(connection)?) {
                return Ok(false);
            }
            save(connection, session, &state)?;
        }
        RestartWrite::Policy { enabled } => {
            let _changed = connection.execute(
                "UPDATE restart_control SET enabled=?1 WHERE singleton=1",
                [enabled],
            )?;
            if !enabled {
                clear_pending(connection)?;
            }
        }
        RestartWrite::Claim(candidate) => return consume_claim(connection, candidate),
        RestartWrite::SuppressEnded { session, next } => {
            return suppress_ended(connection, session, next);
        }
        RestartWrite::Finish { session } => {
            let Some(mut state) = load(connection, session)? else {
                return Ok(false);
            };
            // The old pending intent was consumed at claim. A newer actual
            // turn may already have captured its own shutdown intent; release
            // only this attempt's retention protection, never erase that turn.
            state.claimed = false;
            save(connection, session, &state)?;
        }
    }
    Ok(true)
}
/// Consuming the observed intent is one conditional state transition.
fn consume_claim(connection: &Connection, candidate: RestartCandidate) -> Result<bool, StoreError> {
    let Some(mut state) = load(connection, candidate.session)? else {
        return Ok(false);
    };
    if !policy(connection)?
        || !state.eligible()
        || state.blocked
        || state.claimed
        || state.generation != candidate.generation
        || state.pending.is_none_or(|turn| turn.seq != candidate.turn)
        || !eligible(connection, candidate.session)?
        || select_session(connection, candidate.session)?
            .is_none_or(|info| info.status != SessionStatus::Interrupted)
    {
        return Ok(false);
    }
    state.pending = None;
    state.claimed = true;
    save(connection, candidate.session, &state)?;
    Ok(true)
}

/// A finished run's stop is fenced by its exact durable end boundary.
fn suppress_ended(
    connection: &Connection,
    session: SessionId,
    next: vessel_proto::EventSeq,
) -> Result<bool, StoreError> {
    let Some(info) = select_session(connection, session)? else {
        return Ok(false);
    };
    if info.status.is_live() || info.next_seq != next {
        return Ok(false);
    }
    let Some(mut state) = load(connection, session)? else {
        return Ok(true);
    };
    let generation = state
        .generation
        .next()
        .ok_or(StoreError::RestartGenerationExhausted(session))?;
    state.generation = generation;
    state.blocked = true;
    state.pending = None;
    state.active = None;
    save(connection, session, &state)?;
    Ok(true)
}

/// Pick the next candidate in a stable indexed order; no transcript scan.
pub(crate) fn next(connection: &Connection) -> Result<Option<RestartCandidate>, StoreError> {
    let row: Option<(String,String)> = connection.prepare_cached("SELECT r.session_id,r.state FROM restart_state r
        JOIN sessions s ON s.id=r.session_id WHERE r.pending_at IS NOT NULL AND s.archived_at IS NULL AND s.live=0 AND s.status=?1
        ORDER BY r.pending_at,r.session_id LIMIT 1")?.query_row([encode(&SessionStatus::Interrupted)?], |row| Ok((row.get(0)?, row.get(1)?))).optional()?;
    row.map(|(id, text)| {
        let session = session_id_from_sql(&id)?;
        let state: State = decode(session, Column::Restart, &text)?;
        Ok(state.pending.map(|turn| RestartCandidate {
            session,
            generation: state.generation,
            turn: turn.seq,
        }))
    })
    .transpose()
    .map(Option::flatten)
}
/// Rows decoded per cleanup window, independent of runtime operation quotas.
const SCAN_PAGE_ROWS: i64 = 64;

fn window(
    connection: &Connection,
    after: Option<SessionId>,
    pending_only: bool,
) -> Result<Vec<(SessionId, State)>, StoreError> {
    let mut query = connection.prepare_cached("SELECT session_id,state FROM restart_state WHERE (?1 IS NULL OR session_id>?1)
        AND (pending_at IS NOT NULL OR (?2=0 AND (active_at IS NOT NULL OR claimed=1))) ORDER BY session_id LIMIT ?3")?;
    let mut rows = query.query(params![
        after.map(|id| id.to_string()),
        pending_only,
        SCAN_PAGE_ROWS
    ])?;
    let mut result = Vec::new();
    while let Some(row) = rows.next()? {
        let session = session_id_from_sql(&row.get::<_, String>(0)?)?;
        result.push((
            session,
            decode(session, Column::Restart, &row.get::<_, String>(1)?)?,
        ));
    }
    Ok(result)
}
fn clear_pending(connection: &Connection) -> Result<(), StoreError> {
    let mut after = None;
    loop {
        let batch = window(connection, after, true)?;
        if batch.is_empty() {
            break;
        }
        for (session, mut state) in batch {
            after = Some(session);
            state.pending = None;
            save(connection, session, &state)?;
        }
    }
    Ok(())
}
/// Capture candidates before status normalization or retention, in bounded pages.
pub(crate) fn prepare(connection: &mut Connection, enabled: bool) -> Result<(), StoreError> {
    let transaction = connection.transaction()?;
    let _changed = transaction.execute(
        "UPDATE restart_control SET enabled=?1 WHERE singleton=1",
        [enabled],
    )?;
    let mut after = None;
    loop {
        let batch = window(&transaction, after, false)?;
        if batch.is_empty() {
            break;
        }
        for (session, mut state) in batch {
            after = Some(session);
            let live = select_session(&transaction, session)?
                .is_some_and(|info| info.status == SessionStatus::Running);
            state.pending = if enabled
                && state.eligible()
                && !state.blocked
                && !state.parked
                && eligible(&transaction, session)?
            {
                state
                    .pending
                    .or_else(|| live.then_some(state.active).flatten())
            } else {
                None
            };
            state.active = None;
            state.claimed = false;
            save(&transaction, session, &state)?;
        }
    }
    transaction.commit()?;
    Ok(())
}
/// Generation to install in a reopened actor before it becomes visible.
pub(crate) fn generation(
    connection: &Connection,
    session: SessionId,
) -> Result<RestartGeneration, StoreError> {
    Ok(load(connection, session)?.map_or(RestartGeneration::ZERO, |state| state.generation))
}

pub(crate) fn manually_reopened(
    connection: &Connection,
    session: SessionId,
) -> Result<(), StoreError> {
    if let Some(mut state) = load(connection, session)? {
        state.pending = None;
        state.active = None;
        state.parked = false;
        state.claimed = false;
        save(connection, session, &state)?;
    }
    Ok(())
}
