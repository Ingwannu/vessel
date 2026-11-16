//! A person's marks on a session, and deleting an ended one (protocol 10,
//! ADR 0026): write-side operations run inside the writer's transaction,
//! as `log`'s are, and decoding the marks' columns for every reader.
//!
//! Must not: derive `unread` itself (that is `KeptMarks::marks`, shared
//! with the session runtime), or delete a session that is still live.
//!
//! [Decision Log]
//! - Purpose: the marks are the same for every client and survive the
//!   node, so the store keeps them in the session's row; `session.delete`
//!   removes a session whole.
//! - Each mark is one `UPDATE .. RETURNING` of the columns it changes: one
//!   statement, so the marks it answers are the ones it left, even with
//!   other writes in the same batch. `read` keeps the larger `through`
//!   (`MAX`), so a `read` that arrives late never marks a seen record
//!   unread again, and clears `marked_unread` whatever its `through`, as
//!   "unread until the next read" says. `archive` keeps the first time.
//! - Deleting checks `live` in the same transaction: a session the store
//!   still shows as live (its log not complete) is `SessionLive`, never
//!   deleted under its own appends. Its log, its queue and its attachment
//!   rows go by the foreign keys' cascade; its folder once committed
//!   (`attachments`).

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension as _, Row, params};
use vessel_proto::{EventSeq, KeptMarks, Mark, SessionId};

use crate::Residence;
use crate::codec::{seq_from_sql, seq_to_sql, time_from_sql, time_to_sql};
use crate::error::StoreError;

/// The columns [`StoredMarks::from_row`] reads, in order, from its first
/// column on.
pub(crate) const MARK_COLUMNS: &str = "pinned, archived_at, read_seq, marked_unread";

/// Apply `mark` (made at `at`) to `session`'s row; returns its marks after.
/// The session clock advances as [`Residence`] says for Read and Unread, and
/// for every other accepted mark.
pub(crate) fn mark(
    connection: &Connection,
    session: SessionId,
    mark: Mark,
    at: DateTime<Utc>,
    residence: Residence,
) -> Result<KeptMarks, StoreError> {
    let tick = match (mark, residence) {
        (Mark::Read { .. } | Mark::Unread, Residence::Loaded) => Tick::Never,
        (Mark::Read { .. } | Mark::Unread, Residence::Stored) => {
            Tick::IfAttentionChanges(attention(connection, session)?)
        }
        (Mark::Pin | Mark::Unpin | Mark::Archive | Mark::Unarchive, _) => Tick::Always,
    };
    // What the mark sets, and the value bound to `?2` where it takes one.
    let (set, value) = match mark {
        Mark::Pin => ("pinned = 1", None),
        Mark::Unpin => ("pinned = 0", None),
        Mark::Archive => (
            "archived_at = COALESCE(archived_at, ?2)",
            Some(time_to_sql(at)?),
        ),
        Mark::Unarchive => ("archived_at = NULL", None),
        Mark::Read { through } => (
            "read_seq = MAX(COALESCE(read_seq, 0), ?2), marked_unread = 0",
            Some(seq_to_sql(through)?),
        ),
        Mark::Unread => ("marked_unread = 1", None),
    };
    let mut statement = connection.prepare_cached(&format!(
        "UPDATE sessions SET {set} WHERE id = ?1 RETURNING {MARK_COLUMNS}"
    ))?;
    let id = session.to_string();
    let read = |row: &Row<'_>| StoredMarks::from_row(row, 0);
    let stored = match value {
        Some(value) => statement.query_row(params![id, value], read),
        None => statement.query_row([&id], read),
    }
    .optional()?;
    let kept = stored.ok_or(StoreError::UnknownSession(session))?.kept()?;
    match mark {
        Mark::Archive => {
            let _rows = connection.execute("DELETE FROM inbox WHERE session=?1", [&id])?;
        }
        Mark::Read { .. } => {
            // A late read can clear an explicit unread mark while keeping
            // a farther watermark from a prior read. Apply that kept value
            // to existing rows, just as delayed inserts inherit it.
            let through = kept.read.unwrap_or(EventSeq::ZERO);
            let _rows = connection.execute(
                "UPDATE inbox SET seen=1 WHERE session=?1 AND seq<?2",
                params![id, seq_to_sql(through)?],
            )?;
        }
        Mark::Unread => {
            let _rows = connection.execute("UPDATE inbox SET seen=0 WHERE session=?1", [&id])?;
        }
        Mark::Pin | Mark::Unpin | Mark::Unarchive => {}
    }
    let advance = match tick {
        Tick::Always => true,
        Tick::Never => false,
        Tick::IfAttentionChanges(before) => {
            before.is_some_and(|before| before.unread != kept.marks(before.point).unread)
        }
    };
    if advance {
        let _updated = crate::mutation_time::advance(connection, session)?;
    }
    Ok(kept)
}

/// Whether one mark advances the session clock.
enum Tick {
    Always,
    Never,
    /// `None` for an unknown row, which the mark's own update refuses.
    IfAttentionChanges(Option<Attention>),
}

/// A row's attention before a mark.
#[derive(Clone, Copy)]
struct Attention {
    unread: bool,
    /// The last record asking for attention; a mark does not change it.
    point: Option<EventSeq>,
}

/// `session`'s attention as stored, read in the mark's own savepoint.
fn attention(connection: &Connection, session: SessionId) -> Result<Option<Attention>, StoreError> {
    let stored = connection
        .prepare_cached(&format!(
            "SELECT {MARK_COLUMNS}, attention_seq FROM sessions WHERE id = ?1"
        ))?
        .query_row([session.to_string()], |row| {
            Ok((StoredMarks::from_row(row, 0)?, row.get::<_, Option<i64>>(4)?))
        })
        .optional()?;
    stored
        .map(|(marks, point)| {
            let point = attention_from_sql(point)?;
            Ok(Attention {
                unread: marks.kept()?.marks(point).unread,
                point,
            })
        })
        .transpose()
}

/// Delete an ended session with its log, its marks, its queue and its
/// attachment rows. The caller removes its attachment folder once this is
/// committed.
pub(crate) fn delete(connection: &Connection, session: SessionId) -> Result<(), StoreError> {
    let id = session.to_string();
    let live: Option<bool> = connection
        .prepare_cached("SELECT live FROM sessions WHERE id = ?1")?
        .query_row([&id], |row| row.get(0))
        .optional()?;
    match live {
        None => return Err(StoreError::UnknownSession(session)),
        Some(true) => return Err(StoreError::SessionLive(session)),
        Some(false) => {}
    }
    let _rows = connection
        .prepare_cached("DELETE FROM sessions WHERE id = ?1 AND live = 0")?
        .execute([&id])?;
    tracing::debug!(%session, "deleted a session");
    Ok(())
}

/// A row's [`MARK_COLUMNS`] as stored, before their numbers are checked.
pub(crate) struct StoredMarks {
    pinned: bool,
    archived_at: Option<i64>,
    read: Option<i64>,
    marked_unread: bool,
}

impl StoredMarks {
    /// The marks' columns of `row`, starting at column `first`.
    pub(crate) fn from_row(row: &Row<'_>, first: usize) -> rusqlite::Result<Self> {
        let column = |offset: usize| first.saturating_add(offset);
        Ok(Self {
            pinned: row.get(column(0))?,
            archived_at: row.get(column(1))?,
            read: row.get(column(2))?,
            marked_unread: row.get(column(3))?,
        })
    }

    /// The marks, their numbers checked.
    pub(crate) fn kept(self) -> Result<KeptMarks, StoreError> {
        Ok(KeptMarks {
            pinned: self.pinned,
            archived_at: self.archived_at.map(time_from_sql),
            read: self.read.map(seq_from_sql).transpose()?,
            marked_unread: self.marked_unread,
        })
    }
}

/// The `attention_seq` column (schema 7): the last record asking for
/// attention, `None` before the first.
pub(crate) fn attention_from_sql(value: Option<i64>) -> Result<Option<EventSeq>, StoreError> {
    value.map(seq_from_sql).transpose()
}
