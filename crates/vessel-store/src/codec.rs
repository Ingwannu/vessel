//! Conversions between `vessel-proto` values and `SQLite` columns. Every
//! column the store reads is decoded here, once.

use chrono::{DateTime, Utc};
use rusqlite::Row;
use vessel_proto::{
    EventRecord, EventSeq, HarnessSessionId, OptionChoice, ProfileName, SessionBoard, SessionEvent,
    SessionId, SessionInfo, SessionSpec, SessionStatus, SessionTitle,
};

use crate::config::ByteCount;
use crate::error::{Column, Quantity, StoreError};
use crate::marks::{StoredMarks, attention_from_sql};

pub(crate) fn seq_to_sql(seq: EventSeq) -> Result<i64, StoreError> {
    i64::try_from(seq.get()).map_err(|_too_big| StoreError::OutOfRange(Quantity::Seq))
}

pub(crate) fn seq_from_sql(value: i64) -> Result<EventSeq, StoreError> {
    u64::try_from(value)
        .map(EventSeq::new)
        .map_err(|_negative| StoreError::OutOfRange(Quantity::Seq))
}

pub(crate) fn time_to_sql(time: DateTime<Utc>) -> Result<i64, StoreError> {
    time.timestamp_nanos_opt()
        .ok_or(StoreError::OutOfRange(Quantity::Time))
}

pub(crate) const fn time_from_sql(nanos: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_nanos(nanos)
}

pub(crate) fn bytes_to_sql(bytes: ByteCount) -> Result<i64, StoreError> {
    i64::try_from(bytes.get()).map_err(|_too_big| StoreError::OutOfRange(Quantity::Bytes))
}

pub(crate) fn bytes_from_sql(value: i64) -> Result<ByteCount, StoreError> {
    u64::try_from(value)
        .map(ByteCount::new)
        .map_err(|_negative| StoreError::OutOfRange(Quantity::Bytes))
}

pub(crate) fn session_id_from_sql(text: &str) -> Result<SessionId, StoreError> {
    text.parse().map_err(|source| StoreError::CorruptId {
        text: text.to_owned(),
        source,
    })
}

pub(crate) fn encode<T: serde::Serialize>(value: &T) -> Result<String, StoreError> {
    serde_json::to_string(value).map_err(StoreError::Encode)
}

pub(crate) fn decode<T: serde::de::DeserializeOwned>(
    session: SessionId,
    column: Column,
    text: &str,
) -> Result<T, StoreError> {
    serde_json::from_str(text).map_err(|source| StoreError::Corrupt {
        session,
        column,
        source,
    })
}

/// The columns [`session_from_row`] expects, in order: the marks'
/// (`marks::MARK_COLUMNS`) from [`FIRST_MARK`] on, then `attention_seq`
/// and schema 15's `title_origin`.
pub(crate) const SESSION_COLUMNS: &str = "id, spec, status, created_at, ended_at, next_seq, harness_session, profile, created_by, \
     via, origin, model, title, abilities, pinned, archived_at, read_seq, marked_unread, \
     attention_seq, title_origin";

/// Where the marks' columns start in [`SESSION_COLUMNS`].
const FIRST_MARK: usize = 14;
/// Where `attention_seq` is in [`SESSION_COLUMNS`].
const ATTENTION: usize = 18;
/// Where `title_origin` is in [`SESSION_COLUMNS`] (schema 15).
const TITLE_ORIGIN: usize = 19;

pub(crate) fn session_from_row(row: &Row<'_>) -> Result<SessionInfo, StoreError> {
    let id = session_id_from_sql(&row.get::<_, String>(0)?)?;
    let spec: SessionSpec = decode(id, Column::Spec, &row.get::<_, String>(1)?)?;
    let status: SessionStatus = decode(id, Column::Status, &row.get::<_, String>(2)?)?;
    let mut info = SessionInfo::new(id, spec, time_from_sql(row.get(3)?));
    info.status = status;
    info.ended_at = row.get::<_, Option<i64>>(4)?.map(time_from_sql);
    info.next_seq = seq_from_sql(row.get(5)?)?;
    info.harness_session = row.get::<_, Option<String>>(6)?.map(HarnessSessionId::new);
    // Schema 3. The current profile is the column, not the spec's: a move
    // changes it (`NULL` is the harness's default). A session stored before
    // schema 3 has neither column set, and neither a profile in its spec
    // nor another creator than the owner.
    info.profile = row
        .get::<_, Option<String>>(7)?
        .map(|name| {
            // Stored as the bare name; the name rule is the serde one.
            serde_json::from_value::<ProfileName>(serde_json::Value::String(name)).map_err(
                |source| StoreError::Corrupt {
                    session: id,
                    column: Column::Profile,
                    source,
                },
            )
        })
        .transpose()?;
    if let Some(creator) = row.get::<_, Option<String>>(8)? {
        info.created_by = decode(id, Column::CreatedBy, &creator)?;
    }
    // Schema 4: both `NULL` for this node's own sessions and older ones.
    info.via = row
        .get::<_, Option<String>>(9)?
        .map(|node| {
            decode(
                id,
                Column::Via,
                &serde_json::Value::String(node).to_string(),
            )
        })
        .transpose()?;
    info.origin = row
        .get::<_, Option<String>>(10)?
        .map(|origin| decode(id, Column::Origin, &origin))
        .transpose()?;
    // Schema 5: `NULL` for a session on its harness's own provider.
    info.model = row
        .get::<_, Option<String>>(11)?
        .map(|model| decode(id, Column::Model, &model))
        .transpose()?;
    // Schema 6: the current title, the one truth for it (`SessionInfo`'s
    // docs); `NULL` is none, as for every row stored before schema 6.
    info.title = row
        .get::<_, Option<String>>(12)?
        .map(|title| {
            // Stored as the bare text; the title rule is the serde one, so
            // an edited row that breaks it is `Corrupt`, never a value.
            decode::<SessionTitle>(
                id,
                Column::Title,
                &serde_json::Value::String(title).to_string(),
            )
        })
        .transpose()?;
    // Schema 7: what the agent said it can do in its last handshake, so an
    // ended session says whether it can start again.
    info.abilities = row
        .get::<_, Option<String>>(13)?
        .map(|abilities| decode(id, Column::Abilities, &abilities))
        .transpose()?;
    // Schema 7: the marks, `unread` derived as the session runtime derives
    // it (`KeptMarks::marks`).
    let kept = StoredMarks::from_row(row, FIRST_MARK)?.kept()?;
    info.marks = kept.marks(attention_from_sql(row.get(ATTENTION)?)?);
    info.board = stored_board(&info, kept.is_archived());
    // Schema 15: who produced the title. `NULL` (every row from before
    // it) is unknown, which generation does not replace.
    info.title_origin = row
        .get::<_, Option<String>>(TITLE_ORIGIN)?
        .map(|origin| decode::<vessel_proto::TitleOrigin>(id, Column::TitleOrigin, &origin))
        .transpose()?
        .unwrap_or_default();
    Ok(info)
}

/// A session's `choices` column (schema 6): `None` before its first
/// options report.
pub(crate) fn choices_from_sql(
    session: SessionId,
    text: Option<String>,
) -> Result<Option<Vec<OptionChoice>>, StoreError> {
    text.map(|text| decode(session, Column::Choices, &text))
        .transpose()
}

/// A stored session's place on the board (protocol 8, ADR 0019; ended
/// sessions as ADR 0026 amends it); derived here, never stored.
///
/// [Decision Log]
/// - Purpose: a session served from the store (every session of an earlier
///   run, and this run's once the node handed it over) must be on the
///   board like a live one, in the same place it had when it ended.
/// - Constraint: the reducer lives in `vessel-session`, which this crate
///   must not depend on (ADR 0001's crate map), and the store has none of
///   the facts behind a live placement (turns, permissions, output).
/// - Choice: a stored session that has ended is in the archive once it is
///   archived (`SessionBoard::archived`), and in building as `exited`
///   until then (`SessionBoard::exited`), its last activity its end: the
///   definitions the session runtime uses for an ended session too
///   (`SessionBoard::ended`), so what the session's own info showed at its
///   end is what this gives. A stored session still live is one the node
///   serves from its runtime; the store leaves its board out rather than
///   guess, and a client would place it by its status.
/// - Must stay true: an ended session's place is `SessionBoard::ended`
///   wherever it is built.
fn stored_board(info: &SessionInfo, archived: bool) -> Option<SessionBoard> {
    if info.status.is_live() {
        return None;
    }
    // An ended row always has its end time (the final status sets both);
    // the creation is a floor, should one ever lack it.
    Some(SessionBoard::ended(
        info.ended_at.unwrap_or(info.created_at),
        archived,
    ))
}

/// A stored record and its stored size; the size counts toward page and
/// retention limits.
pub(crate) fn record_from_row(
    session: SessionId,
    row: &Row<'_>,
) -> Result<(EventRecord, usize), StoreError> {
    let text: String = row.get(2)?;
    let event: SessionEvent = decode(session, Column::Event, &text)?;
    let record = EventRecord {
        seq: seq_from_sql(row.get(0)?)?,
        at: time_from_sql(row.get(1)?),
        event,
    };
    Ok((record, text.len()))
}
