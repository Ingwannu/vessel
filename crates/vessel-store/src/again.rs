//! Starting an ended session again (protocol 10's `session.resume`, ADR
//! 0026): what the store keeps of an ended session for it, and reopening
//! its row before the session publishes anything.
//!
//! Must not: append records (the session's run does, through `log`), or
//! reopen a row that is live, gone, or whose log is not where the caller
//! read it.
//!
//! [Decision Log]
//! - Purpose: a session started again continues its own log after its
//!   final status (a final status ends a run, not the log), so retention
//!   must not take its row while it starts, and the row must say it runs
//!   before its next record is written.
//! - The reopening is one `UPDATE .. RETURNING`, as a mark is: `live`, its
//!   status `starting`, no end, and out of the archive (a session started
//!   again is not put away), for a row that has ended and whose log ends
//!   where the caller read it. Retention prunes only ended rows (`live = 0
//!   AND ended_at IS NOT NULL`) and `session.delete` refuses a live one, so
//!   from its commit on the row is the running session's.
//! - Refused, the row unchanged: one deleted or pruned (`UnknownSession`),
//!   one still live (`SessionLive`), one whose log moved since the caller
//!   read it (`OutOfOrder`: the log continues elsewhere than expected).
//! - What a run continues from is read in one read transaction (its info,
//!   choices, queue, last context report and attention point), so the
//!   caller builds the run from one state of the row and reopens with the
//!   `next_seq` it read.
//! - Cost: two statements per start again; nothing on any other path.

use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::{
    ContextUsage, EventSeq, KeptMarks, OptionChoice, ProfileName, PromptQueue, SessionEvent,
    SessionId, SessionInfo, SessionStatus,
};

use crate::codec::{choices_from_sql, decode, encode, seq_from_sql, seq_to_sql};
use crate::error::{Column, StoreError};
use crate::marks::{MARK_COLUMNS, StoredMarks, attention_from_sql};
use crate::reader::select_session;

/// An ended session as the store keeps it, for starting it again
/// (protocol 10's `session.resume`, [`Store::ended_session`]).
///
/// [`Store::ended_session`]: crate::Store::ended_session
#[derive(Debug, Clone)]
pub struct EndedSession {
    /// Its info: id, spec, creator, origin, current profile, title, the
    /// harness session `session/load` takes, its recorded abilities, its
    /// marks, and `next_seq`, where its log continues.
    pub info: SessionInfo,
    /// The choices of its last options report; `None` before any.
    pub choices: Option<Vec<OptionChoice>>,
    /// The prompts that waited when it ended.
    pub queue: PromptQueue,
    /// Its agent's last context report.
    pub usage: Option<ContextUsage>,
    /// The last record of its log that asked for attention, which the
    /// next run's unread starts from.
    pub attention: Option<EventSeq>,
    /// The profiles its files were linked into by moves (`profile_changed`
    /// records, either side; `None` the default), still in its log: its
    /// conversation is one file under each (ADR 0053's claim).
    pub moved_through: BTreeSet<Option<ProfileName>>,
}

/// The columns a run continues from, as stored.
struct Continued {
    choices: Option<String>,
    queue: Option<String>,
    usage: Option<String>,
    attention: Option<i64>,
}

/// `id` as the store keeps it, if stored; ended or not (the caller
/// decides what it may start again).
pub(crate) fn ended_session(
    connection: &Connection,
    id: SessionId,
) -> Result<Option<EndedSession>, StoreError> {
    // One read transaction, so the info and the rest are of one state.
    let transaction = connection.unchecked_transaction()?;
    let Some(info) = select_session(&transaction, id)? else {
        return Ok(None);
    };
    let row: Option<Continued> = transaction
        .prepare_cached("SELECT choices, queue, usage, attention_seq FROM sessions WHERE id = ?1")?
        .query_row([id.to_string()], |row| {
            Ok(Continued {
                choices: row.get(0)?,
                queue: row.get(1)?,
                usage: row.get(2)?,
                attention: row.get(3)?,
            })
        })
        .optional()?;
    let Some(Continued {
        choices,
        queue,
        usage,
        attention,
    }) = row
    else {
        return Ok(None);
    };
    let moved_through = moved_through(&transaction, id)?;
    Ok(Some(EndedSession {
        moved_through,
        choices: choices_from_sql(id, choices)?,
        queue: queue
            .map(|queue| decode(id, Column::Queue, &queue))
            .transpose()?
            .unwrap_or_default(),
        usage: usage
            .map(|usage| decode(id, Column::Usage, &usage))
            .transpose()?,
        attention: attention_from_sql(attention)?,
        info,
    }))
}

/// The profiles `id`'s moves linked its files into: both sides of every
/// `profile_changed` record its log still holds. Only those records are
/// read (they begin with their tag), so a long log is not decoded.
fn moved_through(
    connection: &Connection,
    id: SessionId,
) -> Result<BTreeSet<Option<ProfileName>>, StoreError> {
    let mut statement = connection.prepare_cached(
        "SELECT event FROM events WHERE session_id = ?1 \
         AND event LIKE '{\"type\":\"profile_changed\"%' ORDER BY seq",
    )?;
    let mut rows = statement.query([id.to_string()])?;
    let mut profiles = BTreeSet::new();
    while let Some(row) = rows.next()? {
        let text: String = row.get(0)?;
        if let SessionEvent::ProfileChanged { from, to } = decode(id, Column::Event, &text)? {
            let _from = profiles.insert(from);
            let _to = profiles.insert(to);
        }
    }
    Ok(profiles)
}

/// Reopen `session`'s row for a run whose first record is `next`; its
/// marks after (out of the archive).
pub(crate) fn reopen(
    connection: &Connection,
    session: SessionId,
    next: EventSeq,
) -> Result<KeptMarks, StoreError> {
    let id = session.to_string();
    let reopened = connection
        .prepare_cached(&format!(
            "UPDATE sessions SET live = 1, status = ?3, ended_at = NULL, archived_at = NULL
             WHERE id = ?1 AND live = 0 AND next_seq = ?2
             RETURNING {MARK_COLUMNS}"
        ))?
        .query_row(
            params![id, seq_to_sql(next)?, encode(&SessionStatus::Starting)?],
            |row| StoredMarks::from_row(row, 0),
        )
        .optional()?;
    if let Some(marks) = reopened {
        let _updated = crate::mutation_time::advance(connection, session)?;
        let _rows = connection.execute("DELETE FROM inbox WHERE session=?1", [&id])?;
        tracing::debug!(%session, next = %next, "reopened a session's row to start it again");
        return marks.kept();
    }
    // Why not, read in the same transaction.
    let row: Option<(bool, i64)> = connection
        .prepare_cached("SELECT live, next_seq FROM sessions WHERE id = ?1")?
        .query_row([&id], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()?;
    match row {
        None => Err(StoreError::UnknownSession(session)),
        Some((true, _)) => Err(StoreError::SessionLive(session)),
        Some((false, found)) => Err(StoreError::OutOfOrder {
            session,
            expected: seq_from_sql(found)?,
            got: next,
        }),
    }
}
