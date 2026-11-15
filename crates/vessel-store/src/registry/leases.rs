//! Durable pre-spawn leases fence archive against actor registration races.

use super::{RegistryStartSettlement, WorkspaceStartLease, read, resource_key, write};
use crate::StoreError;
use rusqlite::{Connection, params};
use vessel_proto::WorkspaceId;
use vessel_proto::registry::{RegistryRevision, WorkspaceResource};

pub(super) fn claim(
    connection: &Connection,
    workspace: WorkspaceId,
    resource: WorkspaceResource,
    expected: RegistryRevision,
) -> Result<(), StoreError> {
    write::require_active(connection, workspace)?;
    let mut record = read::workspace(connection, workspace)?.ok_or(StoreError::RegistryConflict)?;
    if record.descriptor.revision != expected {
        return Err(StoreError::RegistryConflict);
    }
    let next = expected
        .next()
        .ok_or(StoreError::RegistryRevisionExhausted)?;
    super::capacity::resource(connection, resource)?;
    let (kind, id) = resource_key(resource);
    let changed=connection.execute("INSERT INTO workspace_registry_resources(kind,resource,workspace,stage,lease_revision) VALUES (?1,?2,?3,'pending',?4) ON CONFLICT(kind,resource) DO UPDATE SET stage='pending',lease_revision=excluded.lease_revision WHERE workspace=excluded.workspace AND stage='failed'",params![kind,id,workspace.to_string(),write::revision(next)?])?;
    if changed == 0 {
        return Err(StoreError::RegistryConflict);
    }
    record.descriptor.revision = next;
    write::put(connection, &record, Some(expected))
}

pub(super) fn settle(
    connection: &Connection,
    lease: &WorkspaceStartLease,
    settlement: RegistryStartSettlement,
) -> Result<(), StoreError> {
    let (kind, id) = resource_key(lease.resource);
    let stage = match settlement {
        RegistryStartSettlement::Ready => "ready",
        RegistryStartSettlement::Failed => "failed",
    };
    let changed=connection.execute("UPDATE workspace_registry_resources SET stage=?4 WHERE kind=?1 AND resource=?2 AND workspace=?3 AND lease_revision=?5 AND stage IN ('pending','uncertain')",params![kind,id,lease.workspace.to_string(),stage,write::revision(lease.revision)?])?;
    if changed == 0 {
        return Err(StoreError::RegistryConflict);
    }
    let mut record =
        read::workspace(connection, lease.workspace)?.ok_or(StoreError::RegistryConflict)?;
    let expected = record.descriptor.revision;
    record.descriptor.revision = expected
        .next()
        .ok_or(StoreError::RegistryRevisionExhausted)?;
    // Archiving admits no new starts, but already-admitted starts must be able to settle.
    write::put(connection, &record, Some(expected))
}

/// A stable terminal may admit its next run only from its exact settled predecessor.
pub(super) fn restart(
    connection: &Connection,
    previous: &WorkspaceStartLease,
    expected: RegistryRevision,
) -> Result<(), StoreError> {
    write::require_active(connection, previous.workspace)?;
    let mut record =
        read::workspace(connection, previous.workspace)?.ok_or(StoreError::RegistryConflict)?;
    if record.descriptor.revision != expected {
        return Err(StoreError::RegistryConflict);
    }
    let next = expected
        .next()
        .ok_or(StoreError::RegistryRevisionExhausted)?;
    let (kind, id) = resource_key(previous.resource);
    let changed=connection.execute("UPDATE workspace_registry_resources SET stage='pending',lease_revision=?4 WHERE kind=?1 AND resource=?2 AND workspace=?3 AND stage='ready' AND lease_revision=?5",params![kind,id,previous.workspace.to_string(),write::revision(next)?,write::revision(previous.revision)?])?;
    if changed == 0 {
        return Err(StoreError::RegistryConflict);
    }
    record.descriptor.revision = next;
    write::put(connection, &record, Some(expected))
}
