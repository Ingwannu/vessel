//! A `SQLite` read transaction keeps descriptor and resource ownership hydration atomic.

use super::{RegistryRecord, RegistryReservation, StoredRegistry, decode};
use crate::StoreError;
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::WorkspaceId;
use vessel_proto::registry::{RegistryRevision, WorkspaceMembership, WorkspaceResource};

pub(crate) fn workspace(
    connection: &Connection,
    id: WorkspaceId,
) -> Result<Option<RegistryRecord>, StoreError> {
    connection
        .query_row(
            "SELECT descriptor, recovery FROM workspace_registry WHERE id=?1",
            [id.to_string()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()?
        .map(|(descriptor, recovery)| {
            Ok(RegistryRecord {
                descriptor: decode(&descriptor)?,
                recovery: recovery.as_deref().map(decode).transpose()?,
            })
        })
        .transpose()
}

pub(crate) fn load(connection: &Connection) -> Result<StoredRegistry, StoreError> {
    // A reader connection is held exclusively while this deferred transaction runs.
    let transaction = connection.unchecked_transaction()?;
    let revision: i64 = transaction.query_row(
        "SELECT revision FROM workspace_registry_meta WHERE singleton=1",
        [],
        |row| row.get(0),
    )?;
    let mut workspaces = Vec::new();
    {
        let mut statement = transaction
            .prepare_cached("SELECT descriptor FROM workspace_registry ORDER BY created_at,id")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let text: String = row.get(0)?;
            workspaces.push(decode(&text)?);
        }
    }
    let mut memberships = super::workspace_memberships(&transaction, None::<WorkspaceId>)?;
    // Placed sessions belong without a row per session (ADR 0106).
    let derived = super::placement::derived_memberships(&transaction, &workspaces, &memberships)?;
    memberships.extend(derived);
    let labels = crate::workspace_labels::registry_labels(&transaction, &workspaces)?;
    transaction.commit()?;
    Ok(StoredRegistry {
        labels,
        revision: RegistryRevision::from(
            u64::try_from(revision).map_err(StoreError::RegistryRevisionRange)?,
        ),
        workspaces,
        memberships,
    })
}

/// Read resource associations on the caller's existing connection/transaction.
/// A `WorkspaceId` selects exactly that workspace; `None` is the complete snapshot.
/// Pending/failed/historical associations remain present, without role/activity
/// inference. The caller separately owns lifecycle/permission/mutation checks.
pub(crate) fn memberships(
    connection: &Connection,
    workspace: impl Into<Option<WorkspaceId>>,
) -> Result<Vec<WorkspaceMembership>, StoreError> {
    let workspace = workspace.into();
    let mut statement = connection.prepare_cached("SELECT workspace,kind,resource FROM workspace_registry_resources WHERE (?1 IS NULL OR workspace=?1) ORDER BY workspace,kind,resource")?;
    let mut rows = statement.query(params![workspace.map(|id| id.to_string())])?;
    let mut memberships = Vec::new();
    while let Some(row) = rows.next()? {
        let (workspace, kind, id): (String, String, String) =
            (row.get(0)?, row.get(1)?, row.get(2)?);
        // This is the SQLite decoding edge; closed domain values leave it typed.
        let resource = match kind.as_str() {
            "session" => WorkspaceResource::Session {
                session: decode(&serde_json::to_string(&id).map_err(StoreError::Encode)?)?,
            },
            "terminal" => WorkspaceResource::Terminal {
                terminal: decode(&serde_json::to_string(&id).map_err(StoreError::Encode)?)?,
            },
            _ => return Err(StoreError::RegistryConflict),
        };
        let workspace = decode(&serde_json::to_string(&workspace).map_err(StoreError::Encode)?)?;
        memberships.push(WorkspaceMembership {
            workspace,
            resource,
        });
    }
    Ok(memberships)
}

pub(crate) fn reservation(
    connection: &Connection,
    id: WorkspaceId,
) -> Result<Option<RegistryReservation>, StoreError> {
    let row: Option<(String, Option<String>, String)> = connection
        .query_row(
            "SELECT intent,worktree,stage FROM workspace_registry_reservations WHERE id=?1",
            params![id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    row.map(|(intent, worktree, stage)| {
        Ok(RegistryReservation {
            workspace: id,
            intent: decode(&intent)?,
            worktree: worktree
                .map(|id| serde_json::to_string(&id).map_err(StoreError::Encode))
                .transpose()?
                .as_deref()
                .map(decode)
                .transpose()?,
            stage: decode(&serde_json::to_string(&stage).map_err(StoreError::Encode)?)?,
        })
    })
    .transpose()
}

pub(crate) fn resource_owner(
    connection: &Connection,
    resource: WorkspaceResource,
) -> Result<Option<WorkspaceId>, StoreError> {
    let (kind, id) = super::resource_key(resource);
    let owner: Option<String> = connection
        .query_row(
            "SELECT workspace FROM workspace_registry_resources WHERE kind=?1 AND resource=?2",
            params![kind, id],
            |row| row.get(0),
        )
        .optional()?;
    owner
        .map(|id| serde_json::to_string(&id).map_err(StoreError::Encode))
        .transpose()?
        .as_deref()
        .map(decode)
        .transpose()
}
