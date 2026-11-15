use super::*;

#[test]
fn one_migration_creates_all_tables_and_rolls_back_atomically() {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("CREATE TABLE sessions(id TEXT PRIMARY KEY);")
        .unwrap();
    {
        let transaction = connection.transaction().unwrap();
        install_staged(&transaction).unwrap();
        let names: Vec<String> = transaction.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'checkpoint_%' ORDER BY name").unwrap()
            .query_map([], |row| row.get(0)).unwrap().collect::<Result<_,_>>().unwrap();
        assert_eq!(
            names,
            [
                "checkpoint_captures",
                "checkpoint_paths",
                "checkpoint_plans",
                "checkpoint_restores"
            ]
        );
        transaction.rollback().unwrap();
    }
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name LIKE 'checkpoint_%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(
        CHECKPOINTS_SCHEMA,
        super::super::schema::migrations::TITLE_ORIGIN_SCHEMA.saturating_add(2)
    );
}
#[test]
#[ignore = "skeleton: wire the typed journal SQL operations before enabling the feature"]
fn duplicate_restore_id_returns_durable_outcome_without_reapplying_paths() {
    // Proof obligation: implement durable idempotency.
}
#[test]
#[ignore = "skeleton: wire candidate publication fencing and restart reconciliation"]
fn expired_candidate_is_never_promoted_by_restart_or_late_storage_completion() {
    // Proof obligation: implement publication recovery.
}
