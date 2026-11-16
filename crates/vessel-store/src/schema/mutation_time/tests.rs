//! Later migration keeps legacy mutation ordering explicitly Unknown.

use super::*;
use crate::schema::{migrate, migrations::MIGRATIONS};
use rusqlite::{Connection, params};
use vessel_proto::SessionId;

#[test]
fn migration_does_not_invent_a_legacy_clock_and_is_idempotent() {
    let mut connection = Connection::open_in_memory().unwrap();
    let previous = MUTATION_TIME_SCHEMA.checked_sub(1).unwrap();
    for sql in &MIGRATIONS[..previous] {
        connection.execute_batch(sql).unwrap();
    }
    connection
        .pragma_update(None, "user_version", i64::try_from(previous).unwrap())
        .unwrap();
    let id = SessionId::random();
    let spec = r#"{"harness":"codex","mode":"structured"}"#;
    let _rows = connection.execute(
        "INSERT INTO sessions (id,spec,status,live,created_at,next_seq,first_seq,retained_bytes) VALUES (?1,?2,'{}',0,0,0,0,0)",
        params![id.to_string(), spec],
    ).unwrap();
    migrate(&mut connection).unwrap();
    migrate(&mut connection).unwrap();
    let (kept, clock): (String, Option<i64>) = connection
        .query_row(
            "SELECT spec,updated_at FROM sessions WHERE id=?1",
            [id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(kept, spec);
    assert_eq!(clock, None);
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        i64::try_from(MIGRATIONS.len()).unwrap()
    );
}
