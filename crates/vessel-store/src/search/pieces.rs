//! Runs and pieces (ADR 0037): records fed in order become messages, a
//! message is cut into pieces of at most [`PIECE_BYTES`], and each piece
//! is written as a plain row waiting for the full-text index. The cursor
//! and when this runs are `index`'s.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::search::MessageRole;
use vessel_proto::{ContentBlock, EventRecord, EventSeq, SessionId};

use super::said::{Said, said, text_of};
use crate::codec::{record_from_row, seq_from_sql, seq_to_sql, time_to_sql};
use crate::error::{Quantity, StoreError};

/// The most text one piece holds; a longer message is several.
pub(crate) const PIECE_BYTES: usize = 8 * 1024;

/// How far back from [`PIECE_BYTES`] a piece's end looks for white space,
/// so a word is cut only when one is longer than that.
const CUT_AT_SPACE: usize = 512;

/// The most records a run spans (its chunks and whatever lies between
/// them), so reading one back from the log is bounded.
pub(super) const RUN_RECORDS: u32 = 512;

/// Records one catch-up step reads from the log.
pub(crate) const CATCH_UP_RECORDS: u32 = 256;

/// Stored bytes of records one catch-up step reads (a tool's output can be
/// hundreds of KiB a record, and each is decoded).
pub(crate) const CATCH_UP_BYTES: usize = 256 * 1024;

/// How far an id is moved forward past ids already taken before a piece
/// is left out (two pieces in one nanosecond: one record's pieces, or two
/// sessions writing at once).
const NUDGES: i64 = 1_024;

/// A run the index has read but not written.
#[derive(Debug)]
pub(crate) struct Run {
    pub(super) role: MessageRole,
    pub(super) first: EventSeq,
    at: DateTime<Utc>,
    text: String,
    records: u32,
}

impl Run {
    pub(super) fn new(role: MessageRole, record: &EventRecord) -> Self {
        Self {
            role,
            first: record.seq,
            at: record.at,
            text: String::new(),
            records: 0,
        }
    }

    pub(super) fn take_in(&mut self, block: &ContentBlock) {
        if let Some(text) = text_of(block) {
            self.text.push_str(text);
        }
        self.passed();
    }

    /// One more record inside the run's span.
    pub(super) fn passed(&mut self) {
        self.records = self.records.saturating_add(1);
    }

    const fn full(&self) -> bool {
        self.text.len() >= PIECE_BYTES || self.records >= RUN_RECORDS
    }
}

/// Feeds records to a session's index, writing each message as it ends.
pub(super) struct Feeder<'c> {
    pub(super) connection: &'c Connection,
    pub(super) session: SessionId,
    pub(super) run: Option<Run>,
}

impl Feeder<'_> {
    pub(super) fn feed(&mut self, record: &EventRecord) -> Result<(), StoreError> {
        match said(&record.event) {
            Said::Prompt(blocks) => {
                self.close()?;
                let text: Vec<&str> = blocks.iter().filter_map(text_of).collect();
                self.write(MessageRole::Prompt, record.seq, record.at, &text.join("\n"))?;
            }
            Said::Chunk(role, block) => {
                if self.run.as_ref().is_some_and(|run| run.role != role) {
                    self.close()?;
                }
                let run = self.run.get_or_insert_with(|| Run::new(role, record));
                run.take_in(block);
                if run.full() {
                    self.close()?;
                }
            }
            Said::Ends => self.close()?,
            Said::Nothing => {
                if let Some(run) = self.run.as_mut() {
                    run.passed();
                    if run.full() {
                        self.close()?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Feed the log's records from `from`, within [`CATCH_UP_RECORDS`] and
    /// [`CATCH_UP_BYTES`]; answers the next record not read. A record that
    /// does not decode is passed over (the log's own readers report it), so
    /// a step always moves on.
    pub(super) fn read_log(&mut self, from: EventSeq) -> Result<EventSeq, StoreError> {
        let mut statement = self.connection.prepare_cached(
            "SELECT seq, at, event FROM events WHERE session_id = ?1 AND seq >= ?2
             ORDER BY seq LIMIT ?3",
        )?;
        let mut rows = statement.query(params![
            self.session.to_string(),
            seq_to_sql(from)?,
            CATCH_UP_RECORDS
        ])?;
        let mut next = from;
        let mut bytes = 0_usize;
        while let Some(row) = rows.next()? {
            let seq = seq_from_sql(row.get(0)?)?;
            match record_from_row(self.session, row) {
                Ok((record, size)) => {
                    self.feed(&record)?;
                    bytes = bytes.saturating_add(size);
                }
                Err(error) => {
                    tracing::warn!(session = %self.session, %seq, %error, "not indexed: a record that does not decode");
                }
            }
            next = seq.next().ok_or(StoreError::OutOfRange(Quantity::Seq))?;
            if bytes >= CATCH_UP_BYTES {
                break;
            }
        }
        Ok(next)
    }

    /// Write the open run, if any.
    fn close(&mut self) -> Result<(), StoreError> {
        if let Some(run) = self.run.take() {
            self.write(run.role, run.first, run.at, &run.text)?;
        }
        Ok(())
    }

    /// Write one message as pieces of at most [`PIECE_BYTES`].
    fn write(
        &self,
        role: MessageRole,
        seq: EventSeq,
        at: DateTime<Utc>,
        text: &str,
    ) -> Result<(), StoreError> {
        let id = time_to_sql(at)?;
        let mut rest = text;
        while !rest.is_empty() {
            let (piece, after) = rest.split_at(cut(rest));
            if !piece.trim().is_empty() {
                insert(self.connection, self.session, seq, role, id, piece)?;
            }
            rest = after;
        }
        Ok(())
    }
}

/// Where the first piece of `text` ends: at most [`PIECE_BYTES`], after
/// the last white space within [`CUT_AT_SPACE`] of that when there is
/// one, else on a character's boundary.
fn cut(text: &str) -> usize {
    if text.len() <= PIECE_BYTES {
        return text.len();
    }
    let boundary = (1..=PIECE_BYTES)
        .rev()
        .find(|&at| text.is_char_boundary(at))
        .unwrap_or(text.len());
    let near = boundary.saturating_sub(CUT_AT_SPACE);
    text.get(near..boundary)
        .and_then(|tail| tail.rfind(char::is_whitespace))
        .map_or(boundary, |space| {
            near.saturating_add(space).saturating_add(1)
        })
}

/// Insert one piece at `id` or the first free id after it, waiting for the
/// full-text index (`indexed = 0`).
fn insert(
    connection: &Connection,
    session: SessionId,
    seq: EventSeq,
    role: MessageRole,
    at: i64,
    text: &str,
) -> Result<(), StoreError> {
    let mut statement = connection.prepare_cached(
        "INSERT OR IGNORE INTO search_messages (id, session_id, seq, role, at, text)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    let session_text = session.to_string();
    let role_text = role_to_sql(role);
    let mut id = at;
    for _ in 0..NUDGES {
        let inserted = statement.execute(params![
            id,
            session_text,
            seq_to_sql(seq)?,
            role_text,
            at,
            text
        ])?;
        if inserted == 1 {
            return Ok(());
        }
        id = id
            .checked_add(1)
            .ok_or(StoreError::OutOfRange(Quantity::Time))?;
    }
    tracing::warn!(%session, %seq, "not indexed: no free id near the message's time");
    Ok(())
}

/// A role as its column holds it.
pub(crate) const fn role_to_sql(role: MessageRole) -> &'static str {
    match role {
        MessageRole::Prompt => "prompt",
        MessageRole::Reply => "reply",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_end_after_white_space_or_on_a_character_boundary() {
        let hangul = "가".repeat(PIECE_BYTES);
        let at = cut(&hangul);
        assert!(at <= PIECE_BYTES && hangul.is_char_boundary(at) && at > 0);
        assert_eq!(cut("short"), 5);
        let words = "word ".repeat(PIECE_BYTES);
        let at = cut(&words);
        assert!(at <= PIECE_BYTES);
        assert!(words.get(..at).unwrap().ends_with(' '), "cut after a space");
    }
}
