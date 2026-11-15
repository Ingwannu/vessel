//! Replaying the append never rewrites a legacy creation request.

use super::migrations::{BACKGROUND_SCHEMA, MIGRATIONS};
use super::*;

#[test]
fn a_legacy_row_is_shown_and_the_append_is_idempotent() {
    let mut connection = Connection::open_in_memory().unwrap();
    let previous = BACKGROUND_SCHEMA.checked_sub(1).unwrap();
    for sql in &MIGRATIONS[..previous] {
        connection.execute_batch(sql).unwrap();
    }
    connection
        .pragma_update(None, "user_version", i64::try_from(previous).unwrap())
        .unwrap();
    let original = r#"{"harness":"codex","mode":"structured"}"#;
    let id = vessel_proto::SessionId::random();
    let inserted = connection.execute(
        "INSERT INTO sessions (id,spec,status,live,created_at,next_seq,first_seq,retained_bytes) VALUES (?1,?2,'{}',0,0,0,0,0)",
        rusqlite::params![id.to_string(), original],
    ).unwrap();
    assert_eq!(inserted, 1);
    migrate(&mut connection).unwrap();
    migrate(&mut connection).unwrap();
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, i64::try_from(MIGRATIONS.len()).unwrap());
    let (spec, listing): (String, String) = connection
        .query_row(
            "SELECT spec,listing FROM sessions WHERE id=?1",
            [id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(spec, original);
    assert_eq!(
        serde_json::from_str::<vessel_proto::SessionListing>(&listing).unwrap(),
        vessel_proto::SessionListing::Shown
    );
}
