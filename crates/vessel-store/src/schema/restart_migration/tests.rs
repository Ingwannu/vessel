//! Legacy running history stays manual; migration never fabricates active intent.
use crate::schema::migrations::RESTART_SCHEMA;
use crate::schema::{migrate, migrations::MIGRATIONS};
use rusqlite::{Connection, params};
use vessel_proto::{
    AgentEvent, ContentBlock, EventSeq, HarnessKind, SessionEvent, SessionId, SessionInfo,
    SessionMode, SessionSpec, SessionStatus,
};

fn before_recovery() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    let before = RESTART_SCHEMA.saturating_sub(1);
    for migration in MIGRATIONS.iter().take(before) {
        connection.execute_batch(migration).unwrap();
    }
    connection
        .pragma_update(None, "user_version", i64::try_from(before).unwrap())
        .unwrap();
    connection
}

fn legacy_running(connection: &Connection) -> (SessionId, SessionEvent) {
    let session = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        ),
        chrono::DateTime::UNIX_EPOCH,
    );
    crate::log::create_session(connection, &session).unwrap();
    let start = SessionEvent::Agent {
        event: AgentEvent::TurnStarted {
            content: vec![ContentBlock::Text {
                text: "synthetic legacy activity".to_owned(),
            }],
        },
    };
    let updated = connection
        .execute(
            "UPDATE sessions SET status=?2,live=1,next_seq=?3 WHERE id=?1",
            params![
                session.id.to_string(),
                serde_json::to_string(&SessionStatus::Running).unwrap(),
                i64::try_from(EventSeq::ZERO.next().unwrap().get()).unwrap(),
            ],
        )
        .unwrap();
    assert_eq!(updated, 1);
    let inserted = connection
        .execute(
            "INSERT INTO events(session_id,seq,at,event) VALUES(?1,0,0,?2)",
            params![
                session.id.to_string(),
                serde_json::to_string(&start).unwrap()
            ],
        )
        .unwrap();
    assert_eq!(inserted, 1);
    (session.id, start)
}

#[test]
fn an_old_running_row_with_a_real_start_event_migrates_without_active_or_pending_intent() {
    let mut connection = before_recovery();
    let (session, start) = legacy_running(&connection);
    migrate(&mut connection).unwrap();
    crate::restart::prepare(&mut connection, true).unwrap();
    let _interrupted =
        crate::recovery::interrupt_live(&mut connection, chrono::Utc::now()).unwrap();
    assert_eq!(crate::restart::next(&connection).unwrap(), None);
    let projected: i64 = connection
        .query_row("SELECT COUNT(*) FROM restart_state", [], |row| row.get(0))
        .unwrap();
    assert_eq!(projected, 0);
    let kept: String = connection
        .query_row(
            "SELECT event FROM events WHERE session_id=?1 AND seq=0",
            [session.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(kept, serde_json::to_string(&start).unwrap());
    assert_eq!(
        crate::reader::select_session(&connection, session)
            .unwrap()
            .unwrap()
            .status,
        SessionStatus::Interrupted
    );
}
