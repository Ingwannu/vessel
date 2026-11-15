//! Transactional SQL operations. The caller supplies the canonical registry
//! adapter; tests use an explicit fake, never an inferred cwd/session owner.

mod read;
#[cfg(test)]
mod tests;
mod write;

pub(super) use read::{catalog, names};
pub(super) use write::{assign, delete, inspect, update};

use crate::StoreError;
use rusqlite::Connection;
use vessel_proto::{WorkspaceId, workspace_labels::CatalogDelta};

/// The integration seam owns admission and invalidation in this same savepoint.
/// The production adapter uses the registry writer's canonical helpers.
pub(super) trait Registry {
    fn require_active(
        &self,
        connection: &Connection,
        workspace: WorkspaceId,
    ) -> Result<(), StoreError>;
    fn changed(&self, connection: &Connection) -> Result<(), StoreError>;
}

pub(super) struct Mutation<T> {
    pub(super) result: T,
    pub(super) catalog: Option<CatalogDelta>,
    /// The real writer maps this to registry watch invalidation after commit.
    /// Incrementing its durable revision alone does not wake subscribers.
    pub(super) assignments: super::plans::AssignmentChange,
}

/// Appended to the shared migration list only after the parent registry lands.
pub(super) const SQL: &str = "
CREATE TABLE workspace_label_catalog (
    key TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    color TEXT NOT NULL
) STRICT, WITHOUT ROWID;
CREATE TABLE workspace_label_assignments (
    workspace TEXT NOT NULL REFERENCES workspace_registry(id) ON DELETE CASCADE,
    key TEXT NOT NULL REFERENCES workspace_label_catalog(key) ON DELETE CASCADE ON UPDATE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    PRIMARY KEY (workspace, key),
    UNIQUE (workspace, position)
) STRICT, WITHOUT ROWID;
CREATE INDEX workspace_label_affected ON workspace_label_assignments(key);
";
