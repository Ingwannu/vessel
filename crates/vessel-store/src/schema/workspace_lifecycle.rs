//! Retained provision provenance, internal archive claims and run pins (ADR 0082).
//! The integrator assigns the version at merge. Released SQL stays append-only.

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
    stage TEXT NOT NULL CHECK (stage IN ('bound','started','deferred_busy','deferred_members','retiring_unpublished','retiring_archive')),
    PRIMARY KEY (schedule,number)
) STRICT, WITHOUT ROWID;
";

#[cfg(test)]
mod tests {
    use super::super::{MIGRATIONS, NOTIFICATION_CLAIMS_SCHEMA, migrate};
    use rusqlite::{Connection, params};

    #[test]
    fn upgrades_schema_24_reservations_without_reallocating_a_backing() {
        let mut connection = Connection::open_in_memory().unwrap();
        for sql in MIGRATIONS.iter().take(NOTIFICATION_CLAIMS_SCHEMA) {
            connection.execute_batch(sql).unwrap();
        }
        let version = i64::try_from(NOTIFICATION_CLAIMS_SCHEMA).unwrap();
        assert_eq!(version, 24);
        connection
            .pragma_update(None, "user_version", version)
            .unwrap();
        for stage in ["reserved", "creating", "ready", "uncertain"] {
            connection
                .execute(
                    "INSERT INTO workspace_registry_reservations VALUES (?1,'intent','backing',?1)",
                    [stage],
                )
                .unwrap();
        }
        migrate(&mut connection).unwrap();
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
