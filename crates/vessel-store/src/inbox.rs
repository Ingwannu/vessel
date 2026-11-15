//! Transactional inbox projection (ADR 0040). It never scans a session's
//! history: bounded tool snapshots supply the request's own display content.
//!
//! [Decision Log]
//! - Purpose: discover attention without replaying all sessions and keep a
//!   pending request even after its source log record is pruned.
//! - Constraints: one store writer; appends and the projection must commit
//!   together, and one wire page must fit below the transport frame limit.
//! - Alternatives: client replay (expensive, conflicting read state), home
//!   replication (another trust boundary), unbounded full tool snapshots.
//! - Choice: indexed rows, sixteen entries per page, at most 2,048 outcomes;
//!   128 recent tools per session plus tools referenced by pending approvals.
//!   Tool bodies above 32 KiB stay in the session, explicitly indicated.
//! - Cost: projection writes at tool/request/outcome boundaries, never on
//!   message/output chunks; a second device sees committed state on refresh.

mod migrate;
mod tools;
use crate::codec::{
    decode, encode, seq_from_sql, seq_to_sql, session_id_from_sql, time_from_sql, time_to_sql,
};
use crate::error::{Column, StoreError};
pub(crate) use migrate::backfill;
use rusqlite::{Connection, params};
use vessel_proto::inbox::{
    InboxCount, InboxCounts, InboxCursor, InboxEntry, InboxItem, InboxKey, InboxPage, InboxRead,
    ListInbox, MarkInbox,
};
use vessel_proto::{AgentEvent, EventRecord, SessionEvent, SessionId, SessionStatus, StopReason};

const PAGE: usize = 16;

pub(crate) fn observe(
    connection: &Connection,
    session: SessionId,
    record: &EventRecord,
) -> Result<(), StoreError> {
    match &record.event {
        SessionEvent::Agent { event } => agent(connection, session, record, event),
        SessionEvent::Status { status } => {
            match status {
                SessionStatus::Starting | SessionStatus::Running => Ok(()),
                SessionStatus::Interrupted => clear_waiting(connection, session),
                SessionStatus::Failed { .. } | SessionStatus::Exited { .. } => {
                    clear_waiting(connection, session)?;
                    let failed = match status {
                        SessionStatus::Failed { .. } => true,
                        SessionStatus::Exited { code } => *code != Some(0),
                        SessionStatus::Starting
                        | SessionStatus::Running
                        | SessionStatus::Interrupted => false,
                    };
                    // A turn ending followed by its process ending is one fact.
                    let exists: bool = connection.query_row(
                        "SELECT EXISTS(SELECT 1 FROM inbox WHERE session=?1 AND waiting=0)",
                        [session.to_string()],
                        |row| row.get(0),
                    )?;
                    if exists {
                        return Ok(());
                    }
                    insert(
                        connection,
                        session,
                        record,
                        None,
                        if failed {
                            &InboxItem::Failed
                        } else {
                            &InboxItem::Finished
                        },
                    )
                }
            }
        }
        SessionEvent::Rewound { .. } => clear_branch(connection, session),
        SessionEvent::Parked { .. }
        | SessionEvent::Woken
        | SessionEvent::Forked { .. }
        | SessionEvent::Terminal(_)
        | SessionEvent::ProfileChanged { .. }
        | SessionEvent::ModelCall(_)
        | SessionEvent::OptionsChanged { .. }
        | SessionEvent::Renamed { .. }
        | SessionEvent::QueueChanged { .. }
        | SessionEvent::CommandsChanged { .. }
        | SessionEvent::UsageChanged { .. } => Ok(()),
    }
}

fn agent(
    connection: &Connection,
    session: SessionId,
    record: &EventRecord,
    event: &AgentEvent,
) -> Result<(), StoreError> {
    match event {
        AgentEvent::ToolCall(tool) => tools::put(connection, session, record.seq, tool.clone()),
        AgentEvent::ToolCallUpdate(update) => {
            tools::update(connection, session, record.seq, update)
        }
        AgentEvent::PermissionRequested(request) => {
            let (tool, detail) = match &request.tool_call {
                Some(call) => tools::get(connection, session, call, request.tool_scope.as_deref())?,
                None => (None, vessel_proto::inbox::InboxDetail::Complete),
            };
            insert(
                connection,
                session,
                record,
                Some(&format!("p{}", request.id)),
                &InboxItem::Permission {
                    request: request.clone(),
                    tool: tool.map(Box::new),
                    detail,
                },
            )
        }
        AgentEvent::QuestionAsked(question) if !question.delivery_state.is_pending() => {
            remove_request(connection, session, &format!("q{}", question.id))
        }
        AgentEvent::QuestionAsked(question) => insert(
            connection,
            session,
            record,
            Some(&format!("q{}", question.id)),
            &InboxItem::Question {
                question: question.clone(),
            },
        ),
        AgentEvent::PermissionResolved { id, .. } => {
            remove_request(connection, session, &format!("p{id}"))
        }
        AgentEvent::QuestionResolved { id, .. } => {
            remove_request(connection, session, &format!("q{id}"))
        }
        AgentEvent::TurnStarted { .. } => clear_branch(connection, session),
        AgentEvent::TurnEnded { reason, .. } => {
            if *reason == StopReason::EndTurn {
                clear_completed_turn_waiting(connection, session)?;
            } else {
                clear_waiting(connection, session)?;
            }
            let item = match reason {
                StopReason::EndTurn => InboxItem::Finished,
                StopReason::MaxTokens | StopReason::MaxTurnRequests => InboxItem::Limit,
                StopReason::Refusal => InboxItem::Failed,
                StopReason::Cancelled => return Ok(()),
            };
            insert(connection, session, record, None, &item)
        }
        AgentEvent::TurnFailed { .. } => {
            clear_waiting(connection, session)?;
            insert(connection, session, record, None, &InboxItem::Failed)
        }
        AgentEvent::Ready { .. }
        | AgentEvent::UserMessageChunk { .. }
        | AgentEvent::MessageChunk { .. }
        | AgentEvent::ThoughtChunk { .. }
        | AgentEvent::Plan { .. }
        | AgentEvent::Notification(_)
        | AgentEvent::Compaction(_)
        | AgentEvent::Plugin(_)
        | AgentEvent::CommandAccepted { .. }
        | AgentEvent::Error { .. }
        | AgentEvent::HarnessLog { .. }
        | AgentEvent::Steered { .. } => Ok(()),
    }
}

// Rewind changes the agent's conversation without deleting its log. Cached
// tools and a completion from the abandoned branch must not remain current.
fn clear_branch(connection: &Connection, session: SessionId) -> Result<(), StoreError> {
    let _rows = connection.execute("DELETE FROM inbox WHERE session=?1 AND coalesce(json_extract(item,'$.question.lifetime'),'turn')<>'session'", [session.to_string()])?;
    let _tools = connection.execute(
        "DELETE FROM inbox_tools WHERE session=?1",
        [session.to_string()],
    )?;
    Ok(())
}

// Only explicitly tagged synthetic plans survive a successful root completion.
// Presentation kind never changes the lifetime of an ordinary native tool request.
fn clear_completed_turn_waiting(
    connection: &Connection,
    session: SessionId,
) -> Result<(), StoreError> {
    let _rows = connection.execute(
        "DELETE FROM inbox WHERE session=?1 AND waiting=1
         AND coalesce(json_extract(item,'$.question.lifetime'),'turn')<>'session'
         AND json_extract(item,'$.request.plan_proposal') IS NULL
         AND json_extract(item,'$.permission.plan_proposal') IS NULL",
        [session.to_string()],
    )?;
    Ok(())
}
fn clear_waiting(connection: &Connection, session: SessionId) -> Result<(), StoreError> {
    let _rows = connection.execute(
        "DELETE FROM inbox WHERE session=?1 AND waiting=1 AND coalesce(json_extract(item,'$.question.lifetime'),'turn')<>'session'",
        [session.to_string()],
    )?;
    Ok(())
}
fn remove_request(
    connection: &Connection,
    session: SessionId,
    request: &str,
) -> Result<(), StoreError> {
    let _rows = connection.execute(
        "DELETE FROM inbox WHERE session=?1 AND request=?2",
        params![session.to_string(), request],
    )?;
    Ok(())
}
fn insert(
    connection: &Connection,
    session: SessionId,
    record: &EventRecord,
    request: Option<&str>,
    item: &InboxItem,
) -> Result<(), StoreError> {
    // A follower can mark a published record before persistence drains it,
    // and archiving waits for publication to end, not for that drain. Read
    // the keeper's marks in this append transaction rather than recreating
    // attention from defaults after an earlier mark already committed.
    let marks = connection
        .query_row(
            &format!(
                "SELECT {} FROM sessions WHERE id=?1",
                crate::marks::MARK_COLUMNS
            ),
            [session.to_string()],
            |row| crate::marks::StoredMarks::from_row(row, 0),
        )?
        .kept()?;
    if marks.is_archived() {
        return Ok(());
    }
    let seen = !marks.marks(Some(record.seq)).unread;
    let mut item = bounded(item)?;
    if matches!(
        item,
        InboxItem::Permission { .. } | InboxItem::Question { .. }
    ) {
        let used: i64 = connection.query_row(
            "SELECT pending_bytes FROM inbox_meta WHERE singleton=1",
            [],
            |row| row.get(0),
        )?;
        let size = i64::try_from(encode(&item)?.len()).unwrap_or(i64::MAX);
        if used.saturating_add(size) > 33_554_432 {
            item = deferred(&item);
        }
    }
    let _rows = connection.execute(
        "INSERT INTO inbox(session,seq,at,request,waiting,item,seen) VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![
            session.to_string(),
            seq_to_sql(record.seq)?,
            time_to_sql(record.at)?,
            request,
            item.waiting(),
            encode(&item)?,
            seen,
        ],
    )?;
    // Only outcomes are evicted. The actor's existing 64-per-kind pending
    // limit bounds requests per live session; no unanswered request is lost.
    let _old = connection.execute("DELETE FROM inbox WHERE cursor IN (SELECT cursor FROM inbox WHERE waiting=0 ORDER BY cursor DESC LIMIT -1 OFFSET 2048)", [])?;
    Ok(())
}

pub(crate) fn mark(connection: &Connection, mark: &MarkInbox) -> Result<(), StoreError> {
    let rows = connection.execute(
        "UPDATE inbox SET seen=?3 WHERE session=?1 AND seq=?2",
        params![
            mark.key.session.to_string(),
            seq_to_sql(mark.key.seq)?,
            mark.read == InboxRead::Read
        ],
    )?;
    if rows == 0 {
        return Err(StoreError::InboxGone);
    }
    Ok(())
}

pub(crate) fn list(connection: &Connection, page: ListInbox) -> Result<InboxPage, StoreError> {
    let before = page.before.map_or(i64::MAX, |cursor| {
        i64::try_from(cursor.get()).unwrap_or(i64::MAX)
    });
    let mut query = connection.prepare_cached("SELECT cursor,session,seq,at,item,seen FROM inbox WHERE cursor < ?1 ORDER BY cursor DESC LIMIT 17")?;
    let mut rows = query.query([before])?;
    let mut entries = Vec::with_capacity(PAGE);
    let mut more = false;
    while let Some(row) = rows.next()? {
        if entries.len() == PAGE {
            more = true;
            break;
        }
        entries.push(entry_from_row(row)?);
    }
    let next = if more {
        entries.last().map(|entry| entry.cursor)
    } else {
        None
    };
    let counts = connection.query_row("SELECT (SELECT COUNT(*) FROM inbox), (SELECT COUNT(*) FROM inbox WHERE waiting=1), (SELECT COUNT(*) FROM inbox WHERE seen=0)", [], |row| Ok((row.get::<_, i64>(0)?,row.get::<_, i64>(1)?,row.get::<_, i64>(2)?)))?;
    let count = |value| {
        u64::try_from(value)
            .map(InboxCount::new)
            .map_err(|_negative| StoreError::OutOfRange(crate::Quantity::Seq))
    };
    Ok(InboxPage {
        entries,
        next,
        counts: InboxCounts {
            total: count(counts.0)?,
            waiting: count(counts.1)?,
            unread: count(counts.2)?,
        },
    })
}

fn entry_from_row(row: &rusqlite::Row<'_>) -> Result<InboxEntry, StoreError> {
    let session = session_id_from_sql(&row.get::<_, String>(1)?)?;
    let item = decode(session, Column::Inbox, &row.get::<_, String>(4)?)?;
    Ok(InboxEntry {
        cursor: InboxCursor::new(
            u64::try_from(row.get::<_, i64>(0)?)
                .map_err(|_negative| StoreError::OutOfRange(crate::Quantity::Seq))?,
        ),
        key: InboxKey {
            session,
            seq: seq_from_sql(row.get(2)?)?,
        },
        title: None,
        project: None,
        project_name: None,
        at: time_from_sql(row.get(3)?),
        read: if row.get::<_, bool>(5)? {
            InboxRead::Read
        } else {
            InboxRead::Unread
        },
        item,
    })
}

pub(crate) fn get(
    connection: &Connection,
    key: InboxKey,
) -> Result<Option<InboxEntry>, StoreError> {
    let mut query = connection.prepare_cached(
        "SELECT cursor,session,seq,at,item,seen FROM inbox WHERE session=?1 AND seq=?2",
    )?;
    let mut rows = query.query(params![key.session.to_string(), seq_to_sql(key.seq)?])?;
    rows.next()?.map(entry_from_row).transpose()
}

// A pathological harness form must not make every inbox page exceed a frame.
// Keep the actor's exact id actionable through its session instead of clipping
// choices, which could change the meaning of a permission.
fn bounded(item: &InboxItem) -> Result<InboxItem, StoreError> {
    if encode(item)?.len() <= 262_144 {
        return Ok(item.clone());
    }
    Ok(deferred(item))
}
fn deferred(item: &InboxItem) -> InboxItem {
    use vessel_proto::inbox::InboxRequest;
    match item {
        InboxItem::Permission { request, .. } => InboxItem::InSession {
            request: InboxRequest::Permission { id: request.id },
            question: None,
            permission: vessel_proto::timeline::PermissionReference::from_request(request),
        },
        InboxItem::Question { question } => InboxItem::InSession {
            request: InboxRequest::Question { id: question.id },
            question: Some(question.into()),
            permission: None,
        },
        InboxItem::InSession { .. }
        | InboxItem::Finished
        | InboxItem::Failed
        | InboxItem::Limit
        | InboxItem::Unknown => item.clone(),
    }
}

#[cfg(test)]
mod tests;
