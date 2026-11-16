//! Receiving SQL-shape probes for W7's candidate, source and final lease joins.
//! They use real schema/log rows; A5 still owes its own receiving caller tests.
use crate::{PushAttention, StoreError, codec, log, schema};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::{
    AgentEvent, Creator, EventRecord, EventSeq, HarnessKind, NodeId, SessionEvent, SessionId,
    SessionInfo, SessionMode, SessionRef, SessionSpec, StopReason,
    inbox::{InboxCursor, InboxKey},
};

// This is the consumer's surrounding eligibility, not another parentage rule.
const SESSION: &str = "s.archived_at IS NULL AND NOT vessel_has_current_parent(s.created_by,s.detach_receipt) AND json_extract(s.spec,'$.side_question_of') IS NULL";

pub(super) fn now() -> DateTime<Utc> {
    DateTime::UNIX_EPOCH
}
pub(super) fn database() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    schema::configure(&db).unwrap();
    schema::migrate(&mut db).unwrap();
    db
}
pub(super) fn parent() -> SessionRef {
    SessionRef {
        node: NodeId::random(),
        session: SessionId::random(),
    }
}
pub(super) fn child(db: &Connection, parent: SessionRef) -> PushAttention {
    let mut info = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        now(),
    );
    info.created_by = Creator::Agent {
        node: parent.node,
        session: parent.session,
    };
    log::create_session(db, &info).unwrap();
    let record = EventRecord {
        seq: EventSeq::ZERO,
        at: now(),
        event: SessionEvent::Agent {
            event: AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                usage: None,
                point: None,
            },
        },
    };
    let _appended = log::append(db, info.id, [&record]).unwrap();
    let raw: i64 = db
        .query_row(
            "SELECT cursor FROM inbox WHERE session=?1",
            [info.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    PushAttention {
        key: InboxKey {
            session: info.id,
            seq: EventSeq::ZERO,
        },
        cursor: InboxCursor::new(u64::try_from(raw).unwrap()),
    }
}
pub(super) fn detach(db: &Connection, source: PushAttention) {
    let transaction = db.unchecked_transaction().unwrap();
    let _receipt = crate::subagents::detach(&transaction, source.key.session, now()).unwrap();
    transaction.commit().unwrap();
}
pub(super) fn candidates(db: &Connection, limit: usize) -> Result<Vec<SessionId>, StoreError> {
    let mut query = db.prepare(&format!("SELECT i.session FROM inbox i JOIN sessions s ON s.id=i.session WHERE i.cursor>?1 AND i.cursor<=?2 AND i.seen=0 AND (i.waiting=1 OR json_extract(i.item,'$.kind') IN ('finished','limit')) AND {SESSION} ORDER BY i.cursor LIMIT ?3"))?;
    let rows = query.query_map(params![0, i64::MAX, limit], |row| row.get::<_, String>(0))?;
    rows.map(|row| codec::session_id_from_sql(&row?)).collect()
}
pub(super) fn current(db: &Connection, source: PushAttention) -> Result<bool, StoreError> {
    db.query_row(&format!("SELECT EXISTS(SELECT 1 FROM inbox i JOIN sessions s ON s.id=i.session WHERE i.cursor=?1 AND i.session=?2 AND i.seq=?3 AND i.seen=0 AND (i.waiting=1 OR json_extract(i.item,'$.kind') IN ('finished','limit','failed')) AND {SESSION})"),
        params![source.cursor.get(), source.key.session.to_string(), codec::seq_to_sql(source.key.seq)?], |row| row.get(0)).map_err(StoreError::from)
}
pub(super) fn admitted(
    db: &Connection,
    lease: &crate::StoredPush,
    source: PushAttention,
) -> Result<bool, StoreError> {
    db.query_row(&format!("SELECT EXISTS(SELECT 1 FROM push_subscriptions p JOIN devices d ON d.id=p.device JOIN inbox i ON i.session=?4 AND i.seq=?5 JOIN sessions s ON s.id=i.session WHERE p.device=?1 AND p.subscription=?2 AND p.expires_at=?3 AND p.expires_at>?6 AND d.revoked_at IS NULL AND EXISTS(SELECT 1 FROM json_each(d.scopes) WHERE value='read') AND i.cursor=?7 AND i.seen=0 AND (i.waiting=1 OR json_extract(i.item,'$.kind') IN ('finished','limit')) AND {SESSION})"),
        params![lease.device.to_string(), codec::encode(&lease.subscription)?, codec::time_to_sql(lease.expires_at)?, source.key.session.to_string(), codec::seq_to_sql(source.key.seq)?, codec::time_to_sql(now())?, source.cursor.get()], |row| row.get(0)).map_err(StoreError::from)
}
