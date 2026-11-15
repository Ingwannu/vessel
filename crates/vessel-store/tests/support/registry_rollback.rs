//! Shared test-only rollback for fixtures with different session/device builders.

pub(crate) fn undo_registry_schema(connection: &rusqlite::Connection) {
    connection
        .execute_batch(
            "DROP TABLE workspace_registry_resources;
         DROP TABLE workspace_registry_reservations;
         DROP TABLE workspace_registry_meta;
         DROP TABLE workspace_registry;",
        )
        .unwrap();
}
