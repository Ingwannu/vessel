//! Answering a search: the reader's side (ADR 0037).
//!
//! Two walks, both newest first by the pieces' ids and both counting what
//! they read, so a page is bounded in work as well as in hits:
//!
//! - through the full-text index, when the query has a word of three
//!   characters or more (the trigram tokenizer's shortest), at most
//!   [`CANDIDATES`] a page;
//! - through the messages themselves, when every word is shorter (no
//!   trigram exists for them) or the search is kept to one session (its
//!   own rows, by the session's index), at most [`SCANNED`] a page.
//!
//! Every candidate is checked here against every word (`snippet::fold`,
//! the same folding the index does, case only), so the index only narrows:
//! a word it cannot serve never widens what matches, and a hit always has
//! its words to mark. A candidate whose session is gone, or whose record
//! retention pruned, is passed over (its row waits for the idle steps).
//!
//! A page that runs out of work before it has its hits ends with `next`
//! at the last piece it read, so the next page goes on from there.

use rusqlite::{Connection, Row};
use vessel_proto::search::{
    MessageRole, SearchCursor, SearchHit, SearchLimit, SearchMessages, SearchPage,
};
use vessel_proto::{EventSeq, SessionId};

use super::snippet::{contains_all, folded, snippet};
use crate::codec::{seq_from_sql, session_id_from_sql, time_from_sql};
use crate::error::StoreError;

/// Candidates the index walk reads a page.
pub(crate) const CANDIDATES: usize = 10_000;

/// Messages the plain walk reads a page.
pub(crate) const SCANNED: usize = 20_000;

/// The shortest word the trigram tokenizer can find.
const TRIGRAM: usize = 3;

/// Through the index, newest first; `kept` is whether the piece's session
/// and record are still there.
const BY_INDEX: &str = "
SELECT m.id, m.session_id, m.seq, m.role, m.at, m.text, s.first_seq IS NOT NULL AND m.seq >= s.first_seq
FROM search_index
JOIN search_messages AS m ON m.id = search_index.rowid
LEFT JOIN sessions AS s ON s.id = m.session_id
WHERE search_index MATCH ?1 AND search_index.rowid < ?2
ORDER BY search_index.rowid DESC";

/// The pieces the index has not taken in yet (written since the writer's
/// last idle step), newest first: walked beside [`BY_INDEX`], so a message
/// is found the moment it is written, not once the index has it. Named
/// index: without statistics the planner would walk every row by id and
/// filter, which no bound of the page's would see.
const WAITING: &str = "
SELECT m.id, m.session_id, m.seq, m.role, m.at, m.text, s.first_seq IS NOT NULL AND m.seq >= s.first_seq
FROM search_messages AS m INDEXED BY search_messages_pending
LEFT JOIN sessions AS s ON s.id = m.session_id
WHERE m.indexed = 0 AND m.id < ?1 ORDER BY m.id DESC";

/// Every message, newest first (the index's rows and those it has not
/// taken in yet).
const EVERY: &str = "
SELECT m.id, m.session_id, m.seq, m.role, m.at, m.text, s.first_seq IS NOT NULL AND m.seq >= s.first_seq
FROM search_messages AS m
LEFT JOIN sessions AS s ON s.id = m.session_id
WHERE m.id < ?1 ORDER BY m.id DESC";

/// One session's messages, newest first.
const ONE_SESSION: &str = "
SELECT m.id, m.session_id, m.seq, m.role, m.at, m.text, s.first_seq IS NOT NULL AND m.seq >= s.first_seq
FROM search_messages AS m
LEFT JOIN sessions AS s ON s.id = m.session_id
WHERE m.session_id = ?2 AND m.id < ?1 ORDER BY m.id DESC";

/// Answer one page.
pub(crate) fn search(
    connection: &Connection,
    request: &SearchMessages,
) -> Result<SearchPage, StoreError> {
    let limit = usize::from(request.limit.unwrap_or(SearchLimit::DEFAULT).get());
    let before = request.before.map_or(i64::MAX, SearchCursor::get);
    let words: Vec<String> = request.query.words().map(folded).collect();
    let long: Vec<&str> = request
        .query
        .words()
        .filter(|word| word.chars().count() >= TRIGRAM)
        .collect();
    let mut page = Page::new(limit, words);
    match (request.session, long.is_empty()) {
        (Some(session), _) => {
            let mut statement = connection.prepare_cached(ONE_SESSION)?;
            let rows = statement.query(rusqlite::params![before, session.to_string()])?;
            page.walk(pieces(rows), SCANNED)?;
        }
        (None, true) => {
            let mut statement = connection.prepare_cached(EVERY)?;
            let rows = statement.query([before])?;
            page.walk(pieces(rows), SCANNED)?;
        }
        (None, false) => {
            let mut indexed = connection.prepare_cached(BY_INDEX)?;
            let mut waiting = connection.prepare_cached(WAITING)?;
            let found = indexed.query(rusqlite::params![fts_query(&long), before])?;
            let unread = waiting.query([before])?;
            page.walk(Newest::new(pieces(found), pieces(unread)), CANDIDATES)?;
        }
    }
    Ok(page.done())
}

/// A statement's rows as pieces.
fn pieces(rows: rusqlite::Rows<'_>) -> impl Iterator<Item = Result<Piece, StoreError>> + '_ {
    rows.and_then(Piece::from_row)
}

/// Two walks newest first, as one (ids are unique across both: a piece is
/// either in the index or waiting for it). A failure comes out first.
struct Newest<A: Iterator, B: Iterator> {
    a: std::iter::Peekable<A>,
    b: std::iter::Peekable<B>,
}

impl<A, B> Newest<A, B>
where
    A: Iterator<Item = Result<Piece, StoreError>>,
    B: Iterator<Item = Result<Piece, StoreError>>,
{
    fn new(a: A, b: B) -> Self {
        Self {
            a: a.peekable(),
            b: b.peekable(),
        }
    }
}

impl<A, B> Iterator for Newest<A, B>
where
    A: Iterator<Item = Result<Piece, StoreError>>,
    B: Iterator<Item = Result<Piece, StoreError>>,
{
    type Item = Result<Piece, StoreError>;

    fn next(&mut self) -> Option<Self::Item> {
        let a_first = match (self.a.peek(), self.b.peek()) {
            (Some(Ok(a)), Some(Ok(b))) => a.id >= b.id,
            (Some(Err(_)) | None, _) => self.a.peek().is_some(),
            (Some(Ok(_)), Some(Err(_))) => false,
            (Some(Ok(_)), None) => true,
        };
        if a_first {
            self.a.next()
        } else {
            self.b.next()
        }
    }
}

/// The index's query: each word as an FTS5 string (so no character of a
/// person's text is read as the query language's), all required.
fn fts_query(words: &[&str]) -> String {
    let quoted: Vec<String> = words
        .iter()
        .map(|word| format!("\"{}\"", word.replace('"', "\"\"")))
        .collect();
    quoted.join(" ")
}

/// A page being filled.
struct Page {
    limit: usize,
    /// The query's words, folded, for checking and for snippets.
    words: Vec<String>,
    hits: Vec<SearchHit>,
    next: Option<SearchCursor>,
    /// The time of the oldest piece read.
    reached: Option<i64>,
}

impl Page {
    fn new(limit: usize, words: Vec<String>) -> Self {
        Self {
            limit,
            words,
            hits: Vec::new(),
            next: None,
            reached: None,
        }
    }

    /// Read pieces until the page is full or `bound` were read.
    fn walk(
        &mut self,
        pieces: impl Iterator<Item = Result<Piece, StoreError>>,
        bound: usize,
    ) -> Result<(), StoreError> {
        let mut read = 0_usize;
        for piece in pieces {
            let piece = piece?;
            read = read.saturating_add(1);
            self.reached = Some(piece.at);
            if piece.kept && contains_all(&folded(&piece.text), &self.words) {
                self.hits.push(piece.hit(&self.words));
                if self.hits.len() >= self.limit {
                    self.next = Some(SearchCursor::new(piece.id));
                    return Ok(());
                }
            }
            if read >= bound {
                self.next = Some(SearchCursor::new(piece.id));
                return Ok(());
            }
        }
        Ok(())
    }

    fn done(self) -> SearchPage {
        SearchPage {
            hits: self.hits,
            reached: self.next.and(self.reached).map(time_from_sql),
            next: self.next,
        }
    }
}

/// One stored piece.
struct Piece {
    id: i64,
    session: SessionId,
    seq: EventSeq,
    role: MessageRole,
    at: i64,
    text: String,
    /// Its session and record are still there.
    kept: bool,
}

impl Piece {
    fn from_row(row: &Row<'_>) -> Result<Self, StoreError> {
        let id: i64 = row.get(0)?;
        let role: String = row.get(3)?;
        Ok(Self {
            id,
            session: session_id_from_sql(&row.get::<_, String>(1)?)?,
            seq: seq_from_sql(row.get(2)?)?,
            role: role_from_sql(id, &role)?,
            at: row.get(4)?,
            text: row.get(5)?,
            kept: row.get(6)?,
        })
    }

    fn hit(&self, words: &[String]) -> SearchHit {
        SearchHit {
            session: self.session,
            seq: self.seq,
            at: time_from_sql(self.at),
            role: self.role,
            snippet: snippet(&self.text, words),
        }
    }
}

fn role_from_sql(id: i64, role: &str) -> Result<MessageRole, StoreError> {
    match role {
        "prompt" => Ok(MessageRole::Prompt),
        "reply" => Ok(MessageRole::Reply),
        _ => Err(StoreError::CorruptMessage { id, what: "role" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(sql: &str, params: &[&dyn rusqlite::ToSql]) -> String {
        let mut connection = Connection::open_in_memory().unwrap();
        crate::schema::configure(&connection).unwrap();
        crate::schema::migrate(&mut connection).unwrap();
        let rows: Vec<String> = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .unwrap()
            .query_map(params, |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        rows.join("\n")
    }

    /// Every walk is a walk of an index, newest first, never a sort of
    /// every match (a page stops after its bound only if nothing sorts
    /// first).
    #[test]
    fn every_walk_reads_in_order_without_sorting() {
        let before: &dyn rusqlite::ToSql = &i64::MAX;
        let words: &dyn rusqlite::ToSql = &"\"abc\"";
        let session: &dyn rusqlite::ToSql = &"a-session";
        for (sql, index, params) in [
            (BY_INDEX, "search_index VIRTUAL TABLE", vec![words, before]),
            (WAITING, "search_messages_pending", vec![before]),
            (EVERY, "INTEGER PRIMARY KEY (rowid<?)", vec![before]),
            (
                ONE_SESSION,
                "search_messages_session",
                vec![before, session],
            ),
        ] {
            let plan = plan(sql, &params);
            assert!(plan.contains(index), "{sql}\n{plan}");
            assert!(!plan.contains("TEMP B-TREE"), "{plan}");
        }
    }

    #[test]
    fn a_person_s_quotes_stay_text() {
        assert_eq!(
            fts_query(&["say \"hi\"", "OR"]),
            "\"say \"\"hi\"\"\" \"OR\""
        );
    }
}
