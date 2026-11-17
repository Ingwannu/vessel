//! Registry-owned backings and pre-publication reservations fence legacy project cleanup.
//! The caller holds canonical project metadata admission across this read and its effects.
//! A `Local` workspace that is Active or Archived owns no directory: legacy removal
//! archives the Active ones in its own savepoint instead (ADR 0106, reversing W11's
//! "every retained lifecycle blocks"). `Managed`, Archiving and Restoring still fence.
use rusqlite::Connection;
use vessel_proto::registry::{WorkspaceBacking, WorkspaceDescriptor, WorkspaceLifecycle};
use vessel_proto::{ProjectId, WorkspaceId};

use super::{WorkspaceProvision, decode};
use crate::StoreError;

pub(crate) fn read(
    connection: &Connection,
    project: ProjectId,
) -> Result<Option<WorkspaceId>, StoreError> {
    let transaction = connection.unchecked_transaction()?;
    let found = find(&transaction, project)?;
    transaction.commit()?;
    Ok(found)
}

fn find(connection: &Connection, project: ProjectId) -> Result<Option<WorkspaceId>, StoreError> {
    {
        let mut statement =
            connection.prepare_cached("SELECT descriptor FROM workspace_registry")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let text: String = row.get(0)?;
            let descriptor: WorkspaceDescriptor = decode(&text)?;
            let unowned = descriptor.backing == WorkspaceBacking::Local
                && matches!(
                    descriptor.lifecycle,
                    WorkspaceLifecycle::Active | WorkspaceLifecycle::Archived { .. }
                );
            if descriptor.project == project && !unowned {
                return Ok(Some(descriptor.id));
            }
        }
    }
    let mut statement =
        connection.prepare_cached("SELECT id,intent FROM workspace_registry_reservations")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let text: String = row.get(1)?;
        let intent: WorkspaceProvision = decode(&text)?;
        if intent.project == project {
            let id: String = row.get(0)?;
            return decode(&serde_json::to_string(&id).map_err(StoreError::Encode)?).map(Some);
        }
    }
    Ok(None)
}
