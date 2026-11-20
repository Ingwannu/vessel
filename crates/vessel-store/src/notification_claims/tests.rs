//! Actual SQL admission, not a mocked claim return value.
use super::*;
use crate::{log, schema};
use vessel_proto::inbox::{InboxCursor, InboxKey, InboxRead, MarkInbox};
use vessel_proto::{
    AgentEvent, EventRecord, EventSeq, HarnessKind, SessionEvent, SessionId, SessionInfo,
    SessionMode, SessionSpec, StopReason,
};

fn database() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    schema::configure(&db).unwrap();
    schema::migrate(&mut db).unwrap();
    db
}
fn source(db: &Connection) -> PushAttention {
    let session = SessionId::random();
    let info = SessionInfo::new(
        session,
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    );
    log::create_session(db, &info).unwrap();
    let record = EventRecord {
        seq: EventSeq::new(0),
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: AgentEvent::TurnEnded {
                usage: None,
                point: None,
                reason: StopReason::EndTurn,
            },
        },
    };
    let _appended = log::append(db, session, [&record]).unwrap();
    let cursor: i64 = db
        .query_row(
            "SELECT cursor FROM inbox WHERE session=?1",
            [session.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    PushAttention {
        key: InboxKey {
            session,
            seq: record.seq,
        },
        cursor: InboxCursor::new(u64::try_from(cursor).unwrap()),
    }
}
#[test]
fn admission_is_once_only_and_read_then_unread_never_reopens_it() {
    let db = database();
    let source = source(&db);
    assert_eq!(
        claim(&db, NotificationAuthority::Owner, source).unwrap(),
        LocalAdmission::Admitted
    );
    assert_eq!(
        claim(&db, NotificationAuthority::Owner, source).unwrap(),
        LocalAdmission::AlreadyClaimed
    );
    crate::inbox::mark(
        &db,
        &MarkInbox {
            key: source.key,
            read: InboxRead::Read,
        },
        crate::Residence::Stored,
    )
    .unwrap();
    assert_eq!(
        claim(&db, NotificationAuthority::Owner, source).unwrap(),
        LocalAdmission::NoAttention
    );
    crate::inbox::mark(
        &db,
        &MarkInbox {
            key: source.key,
            read: InboxRead::Unread,
        },
        crate::Residence::Stored,
    )
    .unwrap();
    assert_eq!(
        claim(&db, NotificationAuthority::Owner, source).unwrap(),
        LocalAdmission::AlreadyClaimed
    );
}
#[test]
fn foreign_cursors_and_a_missing_or_revoked_device_cannot_admit() {
    let db = database();
    let source = source(&db);
    let foreign = PushAttention {
        cursor: InboxCursor::new(source.cursor.get().checked_add(1).unwrap()),
        ..source
    };
    assert_eq!(
        claim(&db, NotificationAuthority::Owner, foreign).unwrap(),
        LocalAdmission::NoAttention
    );
    let device = DeviceId::random();
    assert!(matches!(
        claim(&db, NotificationAuthority::Device(device), source),
        Err(StoreError::NotificationDenied)
    ));
    let _changed = db.execute("INSERT INTO devices(id,label,secret_hash,scopes,created_at) VALUES(?1,'test',X'00','[\"read\"]',0)", [device.to_string()]).unwrap();
    assert!(admissible(&db, NotificationAuthority::Device(device), source).unwrap());
    let _changed = db
        .execute(
            "UPDATE devices SET revoked_at=1 WHERE id=?1",
            [device.to_string()],
        )
        .unwrap();
    assert!(matches!(
        claim(&db, NotificationAuthority::Device(device), source),
        Err(StoreError::NotificationDenied)
    ));
    let _changed = db
        .execute(
            "UPDATE devices SET revoked_at=NULL,scopes='[\"operate\"]' WHERE id=?1",
            [device.to_string()],
        )
        .unwrap();
    assert!(matches!(
        claim(&db, NotificationAuthority::Device(device), source),
        Err(StoreError::NotificationDenied)
    ));
}
#[test]
fn removing_attention_cascades_its_admission_and_old_sources_never_reappear() {
    let db = database();
    let source = source(&db);
    assert_eq!(
        claim(&db, NotificationAuthority::Owner, source).unwrap(),
        LocalAdmission::Admitted
    );
    let _changed = db
        .execute(
            "DELETE FROM inbox WHERE cursor=?1",
            [cursor(source).unwrap()],
        )
        .unwrap();
    let rows: i64 = db
        .query_row("SELECT COUNT(*) FROM notification_claims", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(rows, 0);
    assert_eq!(
        claim(&db, NotificationAuthority::Owner, source).unwrap(),
        LocalAdmission::NoAttention
    );
}

#[test]
fn internal_and_agent_created_attention_remains_readable_without_notification_admission() {
    for internal in [false, true] {
        let db = database();
        let source = source(&db);
        if internal {
            let parent = SessionId::random();
            let _changed = db
                .execute(
                    "UPDATE sessions SET spec=json_set(spec,'$.side_question_of',?1) WHERE id=?2",
                    params![parent.to_string(), source.key.session.to_string()],
                )
                .unwrap();
        } else {
            let creator = vessel_proto::Creator::Agent {
                node: vessel_proto::NodeId::random(),
                session: SessionId::random(),
            };
            let _changed = db
                .execute(
                    "UPDATE sessions SET created_by=?1 WHERE id=?2",
                    params![
                        crate::codec::encode(&creator).unwrap(),
                        source.key.session.to_string()
                    ],
                )
                .unwrap();
        }
        assert_eq!(
            claim(&db, NotificationAuthority::Owner, source).unwrap(),
            LocalAdmission::NoAttention
        );
        assert!(!admissible(&db, NotificationAuthority::Owner, source).unwrap());
        assert!(crate::inbox::get(&db, source.key).unwrap().is_some());
    }
}
