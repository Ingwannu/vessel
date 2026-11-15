//! Shared test-only rollback for fixtures with different session/device builders.

pub(crate) fn undo_registry_schema(connection: &rusqlite::Connection) {
    // Remove only A6's later append before rewinding this predecessor. Older
    // fixture heads legitimately have no labels append; registry assertions stay.
    connection.execute_batch("DROP TABLE IF EXISTS workspace_label_assignments; DROP TABLE IF EXISTS workspace_label_catalog;").unwrap();
    connection
        .execute_batch(
            "DROP TABLE workspace_registry_resources;
         DROP TABLE workspace_registry_reservations;
         DROP TABLE workspace_registry_meta;
         DROP TABLE workspace_registry;",
        )
        .unwrap();
}
