//! Retained budgets are reserved before side effects; history and owner ids never evict.

use crate::StoreError;
use rusqlite::{Connection, params};
use vessel_proto::WorkspaceId;
use vessel_proto::registry::WorkspaceResource;

/// Live workspace/backing identity slots: every record not archived, and every
/// reservation. Archived records do not count (ADR 0082 amendment, ADR 0106).
pub const MAX_WORKSPACES: usize = 256;
/// Resource ownership slots of workspaces that are not archived, including
/// failed and ended starts there (ADR 0082 amendment).
pub const MAX_RESOURCES: usize = 4096;
/// Public descriptor JSON remains bounded so complete snapshots fit one RPC frame.
pub const MAX_DESCRIPTOR_BYTES: usize = 8 * 1024;
/// Retained recovery JSON may include backing safety metadata.
pub const MAX_RECOVERY_BYTES: usize = 256 * 1024;
/// Immutable provision intent JSON, reserved before backing creation.
pub const MAX_INTENT_BYTES: usize = 16 * 1024;
/// Fixed worst-case persisted key/stage/revision bookkeeping per resource.
pub const RESERVED_BYTES_PER_RESOURCE: usize = 256;
/// Fixed key, revision and SQL bookkeeping allowance per workspace.
pub const WORKSPACE_KEY_BYTES: usize = 1024;
/// Worst-case growth is held for every admitted workspace for its full lifetime.
pub const RESERVED_BYTES_PER_WORKSPACE: usize =
    MAX_DESCRIPTOR_BYTES + MAX_RECOVERY_BYTES + MAX_INTENT_BYTES + WORKSPACE_KEY_BYTES;
/// Logical retained payload bound; `SQLite` pages, indexes and WAL are separately managed.
/// Archived `Local` records are pruned beyond their own bound, so they add one term.
pub const MAX_RETAINED_BYTES: usize = (MAX_WORKSPACES + super::MAX_ARCHIVED_WORKSPACES)
    * RESERVED_BYTES_PER_WORKSPACE
    + MAX_RESOURCES * RESERVED_BYTES_PER_RESOURCE;

pub(super) fn workspace(connection: &Connection, id: WorkspaceId) -> Result<(), StoreError> {
    // Existing identities reconcile at capacity; this never allocates a replacement identity.
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT id FROM workspace_registry WHERE id=?1 UNION SELECT id FROM workspace_registry_reservations WHERE id=?1)",
        [id.to_string()], |row| row.get(0),
    )?;
    if !exists {
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM (SELECT id FROM workspace_registry WHERE json_extract(descriptor,'$.lifecycle.kind')!='archived' UNION SELECT id FROM workspace_registry_reservations)",
            [], |row| row.get(0),
        )?;
        if count >= i64::try_from(MAX_WORKSPACES).map_err(StoreError::RegistryRevisionRange)? {
            return Err(StoreError::RegistryFull);
        }
    }
    Ok(())
}

pub(super) fn resource(
    connection: &Connection,
    resource: WorkspaceResource,
) -> Result<(), StoreError> {
    let (kind, id) = super::resource_key(resource);
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM workspace_registry_resources WHERE kind=?1 AND resource=?2)",
        params![kind, id],
        |row| row.get(0),
    )?;
    if !exists {
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM workspace_registry_resources r JOIN workspace_registry w ON w.id=r.workspace WHERE json_extract(w.descriptor,'$.lifecycle.kind')!='archived'",
            [],
            |row| row.get(0),
        )?;
        if count >= i64::try_from(MAX_RESOURCES).map_err(StoreError::RegistryRevisionRange)? {
            return Err(StoreError::RegistryFull);
        }
    }
    Ok(())
}

pub(super) fn bytes(text: &str, limit: usize) -> Result<(), StoreError> {
    if text.len() > limit {
        return Err(StoreError::RegistryRecordTooLarge);
    }
    Ok(())
}
