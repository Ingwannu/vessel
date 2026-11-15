//! Write-side operations on the log, run inside the writer's transaction
//! (or recovery's). They take a plain `Connection` so a caller can wrap
//! each one in its own savepoint.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::{
    AgentAbilities, AgentEvent, ContextUsage, EventRecord, EventSeq, HarnessSessionId,
    OptionChoice, ProfileName, PromptQueue, SessionEvent, SessionId, SessionInfo, SessionOption,
    SessionStatus, SessionTitle, TitleOrigin,
};

use crate::codec::{
    bytes_from_sql, bytes_to_sql, encode, seq_from_sql, seq_to_sql, session_id_from_sql,
    time_to_sql,
};
use crate::config::{ByteCount, Retention};
use crate::error::StoreError;

/// Store a new session with an empty log.
///
/// Only the session's identity, spec, profile, creator, origin, model
/// route, title and current labels come from `info`. Its status is
/// stored as `Starting` and live, and everything else is derived from the
/// records appended later: `info` is a snapshot the runtime took after the
/// process started, and a session that ends within milliseconds would
/// otherwise be stored as ended with no end time and an empty log (and be
/// the first one retention prunes, under the appends still coming).
pub(crate) fn create_session(
    connection: &Connection,
    info: &SessionInfo,
) -> Result<(), StoreError> {
    let _parentage = info
        .parentage()
        .map_err(|_invalid| StoreError::InvalidParentage(info.id))?;
    let inserted = connection
        .prepare_cached(
            "INSERT INTO sessions
                 (id, spec, status, live, created_at, ended_at, next_seq, first_seq,
                  retained_bytes, harness_session, profile, created_by, via, origin, model,
                  title, labels, title_origin, detach_receipt, listing)
             VALUES (?1, ?2, ?3, 1, ?4, NULL, 0, 0, 0, NULL, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT (id) DO NOTHING",
        )?
        .execute(params![
            info.id.to_string(),
            encode(&info.spec)?,
            encode(&SessionStatus::Starting)?,
            time_to_sql(info.created_at)?,
            info.profile.as_ref().map(ProfileName::as_str),
            encode(&info.created_by)?,
            info.via.map(|node| node.to_string()),
            info.origin.as_ref().map(encode).transpose()?,
            info.model.as_ref().map(encode).transpose()?,
            info.title.as_ref().map(SessionTitle::as_str),
            (!info.labels.is_empty())
                .then(|| encode(&info.labels))
                .transpose()?,
            origin_sql(info.title_origin)?,
            info.detach_receipt
                .map(|receipt| encode(&receipt))
                .transpose()?,
            encode(&info.listing)?,
        ])?;
    if inserted == 0 {
        return Err(StoreError::SessionExists(info.id));
    }
    crate::usage::initialize_cost(connection, info)?;
    Ok(())
}

/// What appending changed, for the caller's retention pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Appended {
    /// The session reached a final status in this append.
    pub(crate) ended: bool,
}

/// Whether a run of records moved the session to another profile.
#[derive(Default)]
enum ProfileMove {
    /// No `ProfileChanged` in the run.
    #[default]
    Unchanged,
    /// Moved; `None` is the harness's default profile.
    To(Option<ProfileName>),
}

/// A plan report replaces the projection; a rewind invalidates it even
/// if the old plan event is no longer retained.
#[derive(Default)]
enum PlanMove {
    #[default]
    Unchanged,
    Cleared,
    Reported(crate::LatestPlan),
}

/// The `sessions` columns a run of records changes.
#[derive(Default)]
struct Projection {
    status: Option<SessionStatus>,
    ended_at: Option<DateTime<Utc>>,
    harness_session: Option<HarnessSessionId>,
    /// The last profile change, if any.
    profile: ProfileMove,
    /// The last rename, if any.
    title: Option<(SessionTitle, TitleOrigin)>,
    /// The choices of the last options report, if any.
    choices: Option<Vec<OptionChoice>>,
    /// Protocol 10's projections (schema 7): the last abilities, context
    /// report and queue, and the last record asking for attention.
    abilities: Option<AgentAbilities>,
    usage: Option<ContextUsage>,
    queue: Option<PromptQueue>,
    attention: Option<EventSeq>,
    plan: PlanMove,
    bytes: u64,
}

impl Projection {
    fn observe(&mut self, record: &EventRecord, stored_bytes: usize) {
        let stored_bytes = u64::try_from(stored_bytes).unwrap_or(u64::MAX);
        self.bytes = self.bytes.saturating_add(stored_bytes);
        // The session runtime reads the same rule off the same records, so
        // a live session's unread and its stored row's agree.
        if record.event.asks_for_attention() {
            self.attention = Some(record.seq);
        }
        match &record.event {
            SessionEvent::Status { status } => {
                if !status.is_live() {
                    self.ended_at = Some(record.at);
                }
                self.status = Some(status.clone());
            }
            SessionEvent::Agent {
                event:
                    AgentEvent::Ready {
                        harness_session,
                        abilities,
                    },
            } => {
                self.harness_session = Some(harness_session.clone());
                if abilities.is_some() {
                    self.abilities = *abilities;
                }
            }
            SessionEvent::Agent {
                event: AgentEvent::Plan { entries },
            } => {
                self.plan = PlanMove::Reported(crate::LatestPlan::bounded(entries));
            }
            SessionEvent::UsageChanged { usage } => self.usage = Some(usage.clone()),
            SessionEvent::QueueChanged { queue } => self.queue = Some(queue.clone()),
            SessionEvent::ProfileChanged { to, .. } => {
                self.profile = ProfileMove::To(to.clone());
            }
            SessionEvent::Renamed { title, origin } => {
                self.title = Some((title.clone(), *origin));
            }
            SessionEvent::OptionsChanged { options } => {
                self.choices = Some(options.iter().filter_map(SessionOption::choice).collect());
            }
            // A parked agent is the live session's state only: a node
            // restart interrupts it as any live session (protocol 12).
            SessionEvent::Agent { .. }
            | SessionEvent::Terminal(_)
            | SessionEvent::ModelCall(_)
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            | SessionEvent::Forked { .. } => {}
            // The agent's conversation from a rewind on (protocol 14, ADR
            // 0052): a restarted agent's `ready` is not logged, so a move,
            // a resume or an export after it reads the id from here.
            SessionEvent::Rewound {
                harness_session, ..
            } => {
                self.harness_session = Some(harness_session.clone());
                // The discarded future's plan is no longer current. Older
                // events may be pruned; do not pretend to reconstruct it.
                self.plan = PlanMove::Cleared;
            }
        }
    }
}

/// Append records to a session's log. They must continue it exactly:
/// the first has the session's next sequence number and there are no gaps.
pub(crate) fn append<'a>(
    connection: &Connection,
    session: SessionId,
    records: impl IntoIterator<Item = &'a EventRecord>,
) -> Result<Appended, StoreError> {
    let id = session.to_string();
    let next: i64 = connection
        .prepare_cached("SELECT next_seq FROM sessions WHERE id = ?1")?
        .query_row([&id], |row| row.get(0))
        .optional()?
        .ok_or(StoreError::UnknownSession(session))?;
    let mut expected = seq_from_sql(next)?;
    let mut projection = Projection::default();
    let mut insert = connection.prepare_cached(
        "INSERT INTO events (session_id, seq, at, event) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for record in records {
        if record.seq != expected {
            return Err(StoreError::OutOfOrder {
                session,
                expected,
                got: record.seq,
            });
        }
        let event = encode(&record.event)?;
        let _rows = insert.execute(params![
            id,
            seq_to_sql(record.seq)?,
            time_to_sql(record.at)?,
            event
        ])?;
        crate::usage::note_profile(connection, session, record)?;
        crate::usage::observe(connection, session, record)?;
        crate::inbox::observe(connection, session, record)?;
        projection.observe(record, event.len());
        // Records past u64::MAX cannot exist: the runtime ends a session
        // before its counter would wrap.
        expected = expected
            .next()
            .ok_or(StoreError::OutOfRange(crate::error::Quantity::Seq))?;
    }
    let ended = projection
        .status
        .as_ref()
        .is_some_and(|status| !status.is_live());
    let _rows = connection
        .prepare_cached(
            "UPDATE sessions SET
                 next_seq = ?2,
                 retained_bytes = retained_bytes + ?3,
                 status = COALESCE(?4, status),
                 live = COALESCE(?5, live),
                 ended_at = COALESCE(?6, ended_at),
                 harness_session = COALESCE(?7, harness_session),
                 profile = CASE WHEN ?8 THEN ?9 ELSE profile END,
                 title = COALESCE(?10, title),
                 title_origin = CASE WHEN ?18 THEN ?19 ELSE title_origin END,
                 choices = COALESCE(?11, choices),
                 abilities = COALESCE(?12, abilities),
                 usage = COALESCE(?13, usage),
                 queue = COALESCE(?14, queue),
                 attention_seq = COALESCE(?15, attention_seq),
                 latest_plan = CASE WHEN ?17 THEN ?16 ELSE latest_plan END
             WHERE id = ?1",
        )?
        .execute(params![
            id,
            seq_to_sql(expected)?,
            bytes_to_sql(ByteCount::new(projection.bytes))?,
            projection.status.as_ref().map(encode).transpose()?,
            projection.status.as_ref().map(SessionStatus::is_live),
            projection.ended_at.map(time_to_sql).transpose()?,
            projection
                .harness_session
                .as_ref()
                .map(HarnessSessionId::as_str),
            matches!(projection.profile, ProfileMove::To(_)),
            match &projection.profile {
                ProfileMove::To(Some(name)) => Some(name.as_str()),
                ProfileMove::To(None) | ProfileMove::Unchanged => None,
            },
            projection
                .title
                .as_ref()
                .map(|(title, _origin)| title.as_str()),
            projection.choices.as_ref().map(encode).transpose()?,
            projection.abilities.as_ref().map(encode).transpose()?,
            projection.usage.as_ref().map(encode).transpose()?,
            projection.queue.as_ref().map(encode).transpose()?,
            projection.attention.map(seq_to_sql).transpose()?,
            match &projection.plan {
                PlanMove::Reported(plan) => Some(encode(plan)?),
                PlanMove::Cleared | PlanMove::Unchanged => None,
            },
            !matches!(projection.plan, PlanMove::Unchanged),
            projection.title.is_some(),
            projection
                .title
                .as_ref()
                .map(|(_title, origin)| origin_sql(*origin))
                .transpose()?
                .flatten(),
        ])?;
    crate::usage::prune(connection, Utc::now())?;
    Ok(Appended { ended })
}

/// Delete a session's oldest records until it is within the byte cap.
/// The newest record always stays, so a session's final status is never
/// lost to retention.
pub(crate) fn prune_session(
    connection: &Connection,
    session: SessionId,
    retention: &Retention,
) -> Result<(), StoreError> {
    let id = session.to_string();
    let Some((retained, next)) = connection
        .prepare_cached("SELECT retained_bytes, next_seq FROM sessions WHERE id = ?1")?
        .query_row([&id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .optional()?
    else {
        return Ok(());
    };
    let retained = bytes_from_sql(retained)?.get();
    let Some(excess) = retained.checked_sub(retention.max_session_bytes.get()) else {
        return Ok(());
    };
    if excess == 0 {
        return Ok(());
    }
    let newest = next.saturating_sub(1);
    let mut oldest_first = connection.prepare_cached(
        "SELECT seq, length(CAST(event AS BLOB)) FROM events
         WHERE session_id = ?1 AND seq < ?2 ORDER BY seq",
    )?;
    let mut rows = oldest_first.query(params![id, newest])?;
    let mut freed: u64 = 0;
    let mut cut: Option<i64> = None;
    while freed < excess {
        let Some(row) = rows.next()? else { break };
        cut = Some(row.get(0)?);
        freed = freed.saturating_add(bytes_from_sql(row.get(1)?)?.get());
    }
    drop(rows);
    let Some(cut) = cut else { return Ok(()) };
    let _rows = connection
        .prepare_cached("DELETE FROM events WHERE session_id = ?1 AND seq <= ?2")?
        .execute(params![id, cut])?;
    crate::search::index::pruned(connection, session, cut)?;
    let _rows = connection
        .prepare_cached(
            "UPDATE sessions SET first_seq = ?2, retained_bytes = retained_bytes - ?3
             WHERE id = ?1",
        )?
        .execute(params![
            id,
            cut.saturating_add(1),
            bytes_to_sql(ByteCount::new(freed))?
        ])?;
    tracing::debug!(%session, first_kept = cut.saturating_add(1), freed, "pruned session log");
    Ok(())
}

/// Which ended sessions retention keeps, best first: pinned ones, then
/// those not archived, then archived ones, each newest end first (ADR
/// 0026). Retention deletes from the other end. The order is exactly the
/// `sessions_retention` index's (schema 8), read backwards, so it is a
/// walk of that index, never a sort.
const KEPT_FIRST: &str = "ORDER BY pinned DESC, (archived_at IS NULL) DESC, ended_at DESC, \
                          created_at DESC";

/// Delete the ended sessions beyond the `max_ended_sessions` retention
/// keeps ([`KEPT_FIRST`]), with their logs and attachment rows. Returns
/// the deleted sessions, whose attachment folders the caller removes once
/// this is committed. Only sessions with an end time count: every ended
/// session has one (it comes from its final status record), and requiring
/// it keeps a row in any other state out of reach.
///
/// Bounded: it runs when a session ends, with at most the cap and the
/// sessions that ended since the last run to walk.
pub(crate) fn prune_ended(
    connection: &Connection,
    retention: &Retention,
) -> Result<Vec<SessionId>, StoreError> {
    let mut statement = connection.prepare_cached(&format!(
        "DELETE FROM sessions WHERE id IN (
             SELECT id FROM sessions WHERE live = 0 AND ended_at IS NOT NULL
             {KEPT_FIRST}
             LIMIT -1 OFFSET ?1)
         RETURNING id"
    ))?;
    let mut rows = statement.query([retention.max_ended_sessions])?;
    let mut deleted = Vec::new();
    while let Some(row) = rows.next()? {
        deleted.push(session_id_from_sql(&row.get::<_, String>(0)?)?);
    }
    if !deleted.is_empty() {
        tracing::debug!(deleted = deleted.len(), "pruned ended sessions");
    }
    Ok(deleted)
}

/// Sessions the log still shows as live, with their next sequence number.
pub(crate) fn live_sessions(
    connection: &Connection,
) -> Result<Vec<(SessionId, EventSeq)>, StoreError> {
    let mut statement =
        connection.prepare_cached("SELECT id, next_seq FROM sessions WHERE live = 1")?;
    let mut rows = statement.query([])?;
    let mut live = Vec::new();
    while let Some(row) = rows.next()? {
        let id = session_id_from_sql(&row.get::<_, String>(0)?)?;
        live.push((id, seq_from_sql(row.get(1)?)?));
    }
    Ok(live)
}

/// The origin column: unknown is `NULL`, so an old row and an unrecorded
/// origin read the same and generation leaves both alone.
fn origin_sql(origin: TitleOrigin) -> Result<Option<String>, StoreError> {
    if origin.is_unknown() {
        Ok(None)
    } else {
        encode(&origin).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema;

    /// Retention's order is a walk of its index (schema 8), never a sort of
    /// every ended session.
    #[test]
    fn retention_walks_its_index() {
        let mut connection = Connection::open_in_memory().unwrap();
        schema::configure(&connection).unwrap();
        schema::migrate(&mut connection).unwrap();
        let plan: Vec<String> = connection
            .prepare(&format!(
                "EXPLAIN QUERY PLAN SELECT id FROM sessions
                 WHERE live = 0 AND ended_at IS NOT NULL {KEPT_FIRST} LIMIT -1 OFFSET 3"
            ))
            .unwrap()
            .query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let plan = plan.join("\n");
        assert!(plan.contains("sessions_retention"), "{plan}");
        assert!(!plan.contains("TEMP B-TREE"), "{plan}");
    }
}
