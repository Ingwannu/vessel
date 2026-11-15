//! The nullable append preserves a legacy session and can be run again safely.

use super::{LABELS_SCHEMA, MIGRATIONS, migrate};
use rusqlite::{Connection, params};
use vessel_proto::{HarnessKind, SessionId, SessionInfo, SessionMode, SessionSpec};

fn head_database() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    let head = LABELS_SCHEMA.saturating_sub(1);
    for migration in MIGRATIONS.iter().take(head) {
        connection.execute_batch(migration).unwrap();
    }
    connection
        .pragma_update(None, "user_version", i64::try_from(head).unwrap())
        .unwrap();
    connection
}

#[test]
fn the_label_append_keeps_an_unlabelled_legacy_row_and_is_idempotent() {
    let mut connection = head_database();
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
    let spec = serde_json::to_string(&session.spec).unwrap();
    let status = serde_json::to_string(&session.status).unwrap();
    let changed = connection.execute(
        "INSERT INTO sessions (id, spec, status, live, created_at, next_seq, first_seq, retained_bytes)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5, 0)",
        params![session.id.to_string(), spec, status, session.status.is_live(), i64::try_from(session.next_seq.get()).unwrap()],
    ).unwrap();
    assert_eq!(changed, 1);
    migrate(&mut connection).unwrap();
    migrate(&mut connection).unwrap();
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, i64::try_from(LABELS_SCHEMA).unwrap());
    let (kept, labels): (String, Option<String>) = connection
        .query_row(
            "SELECT spec, labels FROM sessions WHERE id = ?1",
            [session.id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(kept, spec);
    assert_eq!(labels, None);
}
