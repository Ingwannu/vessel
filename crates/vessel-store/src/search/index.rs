//! Reading a session's records into messages (ADR 0037): the writer's
//! side, on an append and in its idle catch-up.
//!
//! Each session has a cursor in its row: `search_seq`, the next record the
//! index has not read, and `search_open`, where the run it has read but not
//! yet written starts (a reply still arriving). The run's text lives in the
//! writer's memory ([`Cached`]) as a copy of what the records
//! `[search_open, search_seq)` say; when the copy is missing or does not
//! match the row (after a restart, a rolled-back append, retention pruning
//! the run's start) it is read again from the log, at most one piece's
//! records. A message that ends is written to `search_messages` as plain
//! rows; the full-text index takes them in later (`idle`).
//!
//! [Decision Log]
//! - Purpose: every message once, as it ends, without slowing the appends
//!   every session makes.
//! - Constraint: the writer is the only thread that writes, and a reply
//!   arrives as many small `message_chunk` records over many batches.
//! - Alternatives: one index row per chunk (a word split over two chunks is
//!   never found, and a snippet is a few tokens); a row rewritten with each
//!   chunk (quadratic in the reply's length); feeding the full-text index
//!   in the append (measured by the contract's review: a commit with a
//!   message cost 14 ms on average and up to a second with the index's
//!   merges, against under 1 ms without, and every session's appends wait
//!   on the writer).
//! - Choice: the open run kept in memory, written once when it ends as a
//!   plain row; the cursor in the session's row, updated in the same
//!   savepoint as the records; only a session whose cursor is at its log's
//!   end is indexed on an append (one behind is left to the idle steps,
//!   which bound their work).
//! - Cost: one point read and one update of the session's row per append,
//!   one row insert per message; a session's open run (at most
//!   [`PIECE_BYTES`](super::pieces::PIECE_BYTES)) in memory while it is open.

use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::{EventRecord, EventSeq, SessionId};

use super::pieces::{Feeder, Run};
use super::said::{Said, said};
use crate::codec::{record_from_row, seq_from_sql, seq_to_sql};
use crate::error::{Quantity, StoreError};

/// A session's open run, as the writer keeps it between batches: valid
/// only while the session's row still says `search_open = run.first` and
/// `search_seq = next`.
#[derive(Debug)]
pub(crate) struct Cached {
    run: Run,
    next: EventSeq,
}

/// What [`advance`] leaves for the writer.
#[derive(Debug)]
pub(crate) struct Advanced {
    /// The open run, to keep for the session's next append.
    pub(crate) cached: Option<Cached>,
    /// The session's index is behind its log: the idle steps go on.
    pub(crate) behind: bool,
}

impl Advanced {
    const BEHIND: Self = Self {
        cached: None,
        behind: true,
    };
}

/// How far [`advance`] may read.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Reach<'a> {
    /// An append: only the records just appended, and only when the index
    /// has read everything before them.
    Fresh(&'a [Arc<EventRecord>]),
    /// An idle step: from the cursor, within the catch-up bounds.
    CatchUp,
}

/// Index a session's records, inside a savepoint of its own: an indexing
/// failure is logged and leaves the cursor where it was (the idle steps
/// try again), never failing the append it rides on.
pub(crate) fn on_append(
    connection: &Connection,
    session: SessionId,
    cached: Option<Cached>,
    fresh: &[Arc<EventRecord>],
) -> Advanced {
    let outcome = connection
        .execute_batch("SAVEPOINT search")
        .map_err(StoreError::from)
        .and_then(|()| advance(connection, session, cached, Reach::Fresh(fresh)));
    let ended = match &outcome {
        Ok(_) => connection.execute_batch("RELEASE search"),
        Err(_) => connection.execute_batch("ROLLBACK TO search; RELEASE search"),
    };
    match (outcome, ended) {
        (Ok(advanced), Ok(())) => advanced,
        (Err(error), _) => {
            tracing::warn!(%session, %error, "not indexed now: left to the idle steps");
            Advanced::BEHIND
        }
        (Ok(_), Err(error)) => {
            tracing::warn!(%session, %error, "not indexed now: left to the idle steps");
            Advanced::BEHIND
        }
    }
}

/// The session's cursor as its row has it.
struct Cursor {
    search_seq: EventSeq,
    search_open: Option<EventSeq>,
    first_seq: EventSeq,
    next_seq: EventSeq,
}

fn cursor(connection: &Connection, session: SessionId) -> Result<Option<Cursor>, StoreError> {
    let row: Option<(i64, Option<i64>, i64, i64)> = connection
        .prepare_cached(
            "SELECT search_seq, search_open, first_seq, next_seq FROM sessions WHERE id = ?1",
        )?
        .query_row([session.to_string()], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .optional()?;
    row.map(|(search_seq, search_open, first_seq, next_seq)| {
        Ok(Cursor {
            search_seq: seq_from_sql(search_seq)?,
            search_open: search_open.map(seq_from_sql).transpose()?,
            first_seq: seq_from_sql(first_seq)?,
            next_seq: seq_from_sql(next_seq)?,
        })
    })
    .transpose()
}

/// Index what the session's log has that the index has not read, as far
/// as `reach` allows, and move the cursor.
pub(crate) fn advance(
    connection: &Connection,
    session: SessionId,
    cached: Option<Cached>,
    reach: Reach<'_>,
) -> Result<Advanced, StoreError> {
    let Some(cursor) = cursor(connection, session)? else {
        return Ok(Advanced {
            cached: None,
            behind: false,
        });
    };
    let from = cursor.search_seq.max(cursor.first_seq);
    if let Reach::Fresh(fresh) = reach
        && fresh.first().is_none_or(|first| first.seq != from)
    {
        // Behind (or nothing new): the idle steps read it from the log.
        return Ok(Advanced {
            cached: None,
            behind: from < cursor.next_seq,
        });
    }
    let run = match (cached, cursor.search_open) {
        (Some(cached), Some(open)) if cached.run.first == open && cached.next == from => {
            Some(cached.run)
        }
        (_, Some(open)) => rebuild(connection, session, open.max(cursor.first_seq), from)?,
        (_, None) => None,
    };
    let mut feeder = Feeder {
        connection,
        session,
        run,
    };
    let read_to = match reach {
        Reach::Fresh(fresh) => {
            let mut read_to = from;
            for record in fresh {
                feeder.feed(record)?;
                read_to = record
                    .seq
                    .next()
                    .ok_or(StoreError::OutOfRange(Quantity::Seq))?;
            }
            read_to
        }
        // A log with nothing from the cursor on (it cannot, as retention
        // keeps the newest record, but a step must always move on).
        Reach::CatchUp => match feeder.read_log(from)? {
            unmoved if unmoved == from => cursor.next_seq,
            read_to => read_to,
        },
    };
    let open = feeder.run.as_ref().map(|run| run.first);
    let _rows = connection
        .prepare_cached("UPDATE sessions SET search_seq = ?2, search_open = ?3 WHERE id = ?1")?
        .execute(params![
            session.to_string(),
            seq_to_sql(read_to)?,
            open.map(seq_to_sql).transpose()?
        ])?;
    Ok(Advanced {
        cached: feeder.run.map(|run| Cached { run, next: read_to }),
        behind: read_to < cursor.next_seq,
    })
}

/// The open run, read again from the log's records `[open, to)` (at most
/// [`RUN_RECORDS`](super::pieces::RUN_RECORDS)): only its chunks are taken in, so nothing is written
/// twice.
fn rebuild(
    connection: &Connection,
    session: SessionId,
    open: EventSeq,
    to: EventSeq,
) -> Result<Option<Run>, StoreError> {
    let mut statement = connection.prepare_cached(
        "SELECT seq, at, event FROM events WHERE session_id = ?1 AND seq >= ?2 AND seq < ?3
         ORDER BY seq",
    )?;
    let mut rows = statement.query(params![
        session.to_string(),
        seq_to_sql(open)?,
        seq_to_sql(to)?
    ])?;
    let mut run: Option<Run> = None;
    while let Some(row) = rows.next()? {
        let Ok((record, _size)) = record_from_row(session, row) else {
            continue;
        };
        match (said(&record.event), run.as_mut()) {
            (Said::Chunk(role, block), None) => {
                let mut opened = Run::new(role, &record);
                opened.take_in(block);
                run = Some(opened);
            }
            (Said::Chunk(role, block), Some(run)) if run.role == role => run.take_in(block),
            (_, Some(run)) => run.passed(),
            (_, None) => {}
        }
    }
    Ok(run)
}

/// Retention removed a session's records through `cut`: a run that started
/// among them now starts after them, so the writer's copy no longer
/// matches the row and is read again from what is left. (Their messages
/// are hidden by `first_seq` and deleted by the idle steps,
/// `search_gone`.)
pub(crate) fn pruned(
    connection: &Connection,
    session: SessionId,
    cut: i64,
) -> Result<(), StoreError> {
    let _rows = connection
        .prepare_cached(
            "UPDATE sessions SET search_open = CASE
                 WHEN search_open IS NULL OR search_open > ?2 THEN search_open
                 WHEN ?2 + 1 < search_seq THEN ?2 + 1
                 ELSE NULL END
             WHERE id = ?1",
        )?
        .execute(params![session.to_string(), cut])?;
    Ok(())
}
