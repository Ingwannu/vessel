//! Exact-source release shares the same CAS/savepoint as lifecycle and revision changes.

use super::{read, write};
use crate::StoreError;
use rusqlite::Connection;
use vessel_proto::registry::{ReleaseWorkspaceAutomation, WorkspaceAutomation};

pub(super) fn release(
    connection: &Connection,
    request: &ReleaseWorkspaceAutomation,
) -> Result<(), StoreError> {
    write::require_active(connection, request.workspace)?;
    let mut record =
        read::workspace(connection, request.workspace)?.ok_or(StoreError::RegistryConflict)?;
    if record.descriptor.revision != request.expected {
        return Err(StoreError::RegistryConflict);
    }
    match &record.descriptor.automation {
        WorkspaceAutomation::Blocked { source } if source == &request.source => {}
        WorkspaceAutomation::Blocked { .. } | WorkspaceAutomation::Allowed => {
            return Err(StoreError::RegistryConflict);
        }
    }
    record.descriptor.automation = WorkspaceAutomation::Allowed;
    record.descriptor.revision = request
        .expected
        .next()
        .ok_or(StoreError::RegistryRevisionExhausted)?;
    write::put(connection, &record, Some(request.expected))
}
