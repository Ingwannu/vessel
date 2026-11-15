//! Shared test-only rollback for fixtures with different session/device builders.

pub(crate) fn undo_registry_schema(connection: &rusqlite::Connection) {
    connection
        .execute_batch(
            "DROP TABLE checkpoint_allocations; DROP TABLE checkpoint_garbage; DROP TABLE checkpoint_paths; DROP TABLE checkpoint_restores; DROP TABLE checkpoint_plans; DROP TABLE checkpoint_captures; DROP TABLE workspace_registry_resources;
         DROP TABLE workspace_registry_reservations;
         DROP TABLE workspace_registry_meta;
         DROP TABLE workspace_registry;",
        )
        .unwrap();
}
