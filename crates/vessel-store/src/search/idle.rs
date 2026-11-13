//! The writer's idle work for the index (ADR 0037): taking new messages
//! into the full-text index, deleting what removed sessions and pruned
//! records left, and catching up sessions whose index is behind their log.
//!
//! Each step is one transaction doing one bounded piece of one kind of
//! work, so a write that arrives meanwhile waits for at most one step. The
//! writer runs steps only while its queue is empty and some work may be
//! left ([`Indexer::maybe`]); a step that finds nothing, or fails, clears
//! that until the next commit, so the writer never spins on work it cannot
//! do.

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension as _, Transaction, params};
use vessel_proto::SessionId;

use super::index::{Advanced, Cached, Reach, advance};
use crate::codec::session_id_from_sql;
use crate::error::StoreError;

/// Text one step takes into the full-text index, or takes out of it (the
/// index tokenises a row's text again to remove it). Bounds a step's time,
/// and with it the index's own merging, which is proportional to what a
/// transaction writes.
const STEP_BYTES: usize = 32 * 1024;

/// Rows one step takes in or deletes.
const STEP_ROWS: i64 = 128;

/// The writer's side of the index between batches: each session's open run,
/// and whether idle work may be left.
#[derive(Debug)]
pub(crate) struct Indexer {
    runs: HashMap<SessionId, Cached>,
    maybe: bool,
}

impl Indexer {
    /// A fresh indexer, which looks for work at the first idle moment
    /// (sessions stored before the index existed, rows an earlier run left).
    pub(crate) fn new() -> Self {
        Self {
            runs: HashMap::new(),
            maybe: true,
        }
    }

    /// Whether the writer should spend idle time on the index.
    pub(crate) const fn maybe(&self) -> bool {
        self.maybe
    }

    /// A batch committed: what it wrote may have left work (a message to
    /// index, a session removed, a session behind).
    pub(crate) const fn committed(&mut self) {
        self.maybe = true;
    }

    /// Take a session's open run for an append's savepoint; hand it back
    /// with [`Indexer::keep`] once that committed.
    pub(crate) fn take(&mut self, session: SessionId) -> Option<Cached> {
        self.runs.remove(&session)
    }

    /// Keep what an append or a step left.
    pub(crate) fn keep(&mut self, session: SessionId, advanced: Advanced) {
        if let Some(cached) = advanced.cached {
            let _replaced = self.runs.insert(session, cached);
        }
    }

    /// A commit failed: no copy can be trusted to match the rows.
    pub(crate) fn forget_all(&mut self) {
        self.runs.clear();
    }

    /// One idle step in a transaction of its own.
    pub(crate) fn step(&mut self, connection: &mut Connection) {
        let stepped = connection
            .transaction()
            .map_err(StoreError::from)
            .and_then(|transaction| {
                let done = self.one(&transaction)?;
                transaction.commit()?;
                Ok(done)
            });
        match stepped {
            Ok(Done::Indexed { rows }) => tracing::trace!(rows, "indexed messages"),
            Ok(Done::Deleted { rows }) => tracing::trace!(rows, "deleted removed messages"),
            Ok(Done::CaughtUp(session, advanced)) => {
                tracing::trace!(%session, behind = advanced.behind, "read a session's log");
                self.keep(session, advanced);
            }
            Ok(Done::Nothing) => self.maybe = false,
            Err(error) => {
                tracing::warn!(%error, "the search index's idle step failed; tried again after the next write");
                self.forget_all();
                self.maybe = false;
            }
        }
    }

    /// The first kind of work there is, newest messages first.
    fn one(&mut self, transaction: &Transaction<'_>) -> Result<Done, StoreError> {
        let rows = take_in(transaction)?;
        if rows > 0 {
            return Ok(Done::Indexed { rows });
        }
        let rows = delete_gone(transaction)?;
        if rows > 0 {
            return Ok(Done::Deleted { rows });
        }
        let Some(session) = behind(transaction)? else {
            return Ok(Done::Nothing);
        };
        let cached = self.runs.remove(&session);
        let advanced = advance(transaction, session, cached, Reach::CatchUp)?;
        Ok(Done::CaughtUp(session, advanced))
    }
}

/// What a step did.
enum Done {
    Indexed { rows: usize },
    Deleted { rows: usize },
    CaughtUp(SessionId, Advanced),
    Nothing,
}

/// Take waiting rows into the full-text index, within the step's bounds.
fn take_in(connection: &Connection) -> Result<usize, StoreError> {
    let mut waiting = connection.prepare_cached(
        "SELECT id, text FROM search_messages WHERE indexed = 0 ORDER BY id LIMIT ?1",
    )?;
    let mut rows = waiting.query([STEP_ROWS])?;
    let mut add =
        connection.prepare_cached("INSERT INTO search_index (rowid, text) VALUES (?1, ?2)")?;
    let mut mark =
        connection.prepare_cached("UPDATE search_messages SET indexed = 1 WHERE id = ?1")?;
    let (mut done, mut bytes) = (0_usize, 0_usize);
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let text: String = row.get(1)?;
        let _added = add.execute(params![id, text])?;
        let _marked = mark.execute([id])?;
        done = done.saturating_add(1);
        bytes = bytes.saturating_add(text.len());
        if bytes >= STEP_BYTES {
            break;
        }
    }
    Ok(done)
}

/// Delete rows a removed session or pruned records left, within the step's
/// bounds; a session's mark goes once its rows are gone.
fn delete_gone(connection: &Connection) -> Result<usize, StoreError> {
    let gone: Option<(String, i64)> = connection
        .prepare_cached("SELECT session_id, through FROM search_gone LIMIT 1")?
        .query_row([], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()?;
    let Some((session, through)) = gone else {
        return Ok(0);
    };
    let mut doomed = connection.prepare_cached(
        "SELECT id, text, indexed FROM search_messages
         WHERE session_id = ?1 AND seq <= ?2 LIMIT ?3",
    )?;
    let mut rows = doomed.query(params![session, through, STEP_ROWS])?;
    let mut unindex = connection.prepare_cached(
        "INSERT INTO search_index (search_index, rowid, text) VALUES ('delete', ?1, ?2)",
    )?;
    let mut delete = connection.prepare_cached("DELETE FROM search_messages WHERE id = ?1")?;
    let (mut done, mut bytes, mut stopped) = (0_usize, 0_usize, false);
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let text: String = row.get(1)?;
        let indexed: bool = row.get(2)?;
        if indexed {
            let _unindexed = unindex.execute(params![id, text])?;
        }
        let _deleted = delete.execute([id])?;
        done = done.saturating_add(1);
        bytes = bytes.saturating_add(text.len());
        if bytes >= STEP_BYTES || i64::try_from(done).unwrap_or(i64::MAX) >= STEP_ROWS {
            stopped = true;
            break;
        }
    }
    drop(rows);
    if !stopped {
        // Only the mark read: a later prune may have raised it meanwhile.
        let _forgotten = connection
            .prepare_cached("DELETE FROM search_gone WHERE session_id = ?1 AND through = ?2")?
            .execute(params![session, through])?;
        return Ok(done.max(1));
    }
    Ok(done)
}

/// The newest session whose index is behind its log.
fn behind(connection: &Connection) -> Result<Option<SessionId>, StoreError> {
    let id: Option<String> = connection
        .prepare_cached(
            "SELECT id FROM sessions WHERE search_seq < next_seq
             ORDER BY created_at DESC LIMIT 1",
        )?
        .query_row([], |row| row.get(0))
        .optional()?;
    id.as_deref().map(session_id_from_sql).transpose()
}
