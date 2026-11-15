//! Exact unsettled actor admissions for lifecycle draining and crash reconciliation.

use super::{PendingRegistryStart, RegistryStartState, WorkspaceStartLease, decode};
use crate::StoreError;
use rusqlite::{Connection, params};
use vessel_proto::WorkspaceId;
use vessel_proto::registry::RegistryRevision;

pub(crate) fn pending(
    connection: &Connection,
    workspace: WorkspaceId,
) -> Result<Vec<PendingRegistryStart>, StoreError> {
    let mut statement=connection.prepare_cached("SELECT kind,resource,lease_revision,stage FROM workspace_registry_resources WHERE workspace=?1 AND stage IN ('pending','uncertain') ORDER BY kind,resource")?;
    let mut rows = statement.query(params![workspace.to_string()])?;
    let mut starts = Vec::new();
    while let Some(row) = rows.next()? {
        let (kind, id, revision, stage): (String, String, i64, String) =
            (row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?);
        let quoted = serde_json::to_string(&id).map_err(StoreError::Encode)?;
        let resource = match kind.as_str() {
            "session" => vessel_proto::registry::WorkspaceResource::Session {
                session: decode(&quoted)?,
            },
            "terminal" => vessel_proto::registry::WorkspaceResource::Terminal {
                terminal: decode(&quoted)?,
            },
            _ => return Err(StoreError::RegistryConflict),
        };
        let admission = match stage.as_str() {
            "pending" => RegistryStartState::Pending,
            "uncertain" => RegistryStartState::Uncertain,
            _ => return Err(StoreError::RegistryConflict),
        };
        let revision = RegistryRevision::from(
            u64::try_from(revision).map_err(StoreError::RegistryRevisionRange)?,
        );
        starts.push(PendingRegistryStart {
            lease: WorkspaceStartLease::new(workspace, resource, revision),
            state: admission,
        });
    }
    Ok(starts)
}
