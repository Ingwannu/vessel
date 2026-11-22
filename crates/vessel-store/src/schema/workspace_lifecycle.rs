//! Retained provision provenance, internal archive claims and run pins (ADR 0082).
//! Schema 26 follows startup recovery. Released SQL stays append-only.

pub(super) const SQL: &str = "
ALTER TABLE workspace_registry_reservations RENAME TO workspace_reservations_previous;
CREATE TABLE workspace_registry_reservations (
    id TEXT NOT NULL PRIMARY KEY,
    intent TEXT NOT NULL,
    worktree TEXT,
    stage TEXT NOT NULL CHECK (stage IN ('reserved','creating','ready','uncertain','retired')),
    retired_at INTEGER,
    CHECK ((stage = 'retired') = (retired_at IS NOT NULL))
) STRICT, WITHOUT ROWID;
INSERT INTO workspace_registry_reservations (id,intent,worktree,stage)
    SELECT id,intent,worktree,stage FROM workspace_reservations_previous;
DROP TABLE workspace_reservations_previous;
CREATE INDEX workspace_reservation_backing ON workspace_registry_reservations(worktree);
ALTER TABLE workspace_registry_resources ADD COLUMN joined INTEGER NOT NULL DEFAULT 0 CHECK (joined IN (0,1));
CREATE TABLE workspace_archive_claims (
    workspace TEXT NOT NULL PRIMARY KEY REFERENCES workspace_registry(id) ON DELETE CASCADE,
    token TEXT NOT NULL,
    purpose TEXT NOT NULL CHECK (purpose IN ('explicit','automatic')),
    defer TEXT CHECK (defer IN ('busy_session','other_members')),
    CHECK (purpose = 'automatic' OR defer IS NULL)
) STRICT, WITHOUT ROWID;
CREATE TABLE schedule_run_bindings (
    schedule TEXT NOT NULL,
    number INTEGER NOT NULL CHECK (number > 0),
    workspace TEXT NOT NULL UNIQUE,
    session TEXT NOT NULL UNIQUE,
    after TEXT NOT NULL CHECK (after IN ('archive','keep')),
    release_requested INTEGER NOT NULL DEFAULT 0 CHECK (release_requested IN (0,1)),
    stage TEXT NOT NULL CHECK (stage IN ('bound','started','kept','deferred_busy','deferred_members','retiring_unpublished','retiring_archive','unconfirmed')),
    PRIMARY KEY (schedule,number)
) STRICT, WITHOUT ROWID;
";

#[cfg(test)]
mod tests {
    use super::super::{
        MIGRATIONS, NOTIFICATION_CLAIMS_SCHEMA, RESTART_SCHEMA, WORKSPACE_LIFECYCLE_SCHEMA, migrate,
    };
    use rusqlite::{Connection, params};

    #[test]
    fn upgrades_schema_24_and_25_reservations_without_reallocating_a_backing() {
        assert_eq!((NOTIFICATION_CLAIMS_SCHEMA, RESTART_SCHEMA), (24, 25));
        assert_eq!(WORKSPACE_LIFECYCLE_SCHEMA, 26);
        for previous in [NOTIFICATION_CLAIMS_SCHEMA, RESTART_SCHEMA] {
            upgrade(previous);
        }
    }

    fn upgrade(previous: usize) {
        let mut connection = Connection::open_in_memory().unwrap();
        for sql in MIGRATIONS.iter().take(previous) {
            connection.execute_batch(sql).unwrap();
        }
        connection
            .pragma_update(None, "user_version", i64::try_from(previous).unwrap())
            .unwrap();
        for stage in ["reserved", "creating", "ready", "uncertain"] {
            let _copied = connection
                .execute(
                    "INSERT INTO workspace_registry_reservations VALUES (?1,'intent','backing',?1)",
                    [stage],
                )
                .unwrap();
        }
        migrate(&mut connection).unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                .unwrap(),
            u32::try_from(MIGRATIONS.len()).unwrap()
        );
        for query in [
            "SELECT COUNT(*) FROM workspace_archive_claims",
            "SELECT COUNT(*) FROM schedule_run_bindings",
        ] {
            assert_eq!(
                connection
                    .query_row(query, [], |row| row.get::<_, u32>(0))
                    .unwrap(),
                0
            );
        }
        for stage in ["reserved", "creating", "ready", "uncertain"] {
            let saved: (String, String, String, Option<i64>) = connection
                .query_row(
                    "SELECT intent,worktree,stage,retired_at FROM workspace_registry_reservations WHERE id=?1",
                    [stage],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .unwrap();
            assert_eq!(
                saved,
                ("intent".into(), "backing".into(), stage.into(), None)
            );
        }
        assert!(
            connection
                .execute(
                    "UPDATE workspace_registry_reservations SET stage='retired' WHERE id='ready'",
                    [],
                )
                .is_err()
        );
        assert_eq!(connection.execute(
            "UPDATE workspace_registry_reservations SET stage='retired',retired_at=?1 WHERE id=?2",
            params![1, "ready"],
        ).unwrap(), 1);
    }
}
