//! Project removal fences all outstanding registry ownership. A worktree
//! removal fences only the exact backing (ADR 0082).
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::registry::{WorkspaceBacking, WorkspaceDescriptor, WorkspaceLifecycle};
use vessel_proto::{ProjectId, WorkspaceId, WorktreeId};

use super::decode;
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

/// The same fence that the node reads before entering the storage write.
pub(crate) fn find(
    connection: &Connection,
    project: ProjectId,
) -> Result<Option<WorkspaceId>, StoreError> {
    let mut statement = connection.prepare_cached(
        "SELECT w.descriptor,r.stage FROM workspace_registry w
         LEFT JOIN workspace_registry_reservations r ON r.id=w.id WHERE w.project=?1",
    )?;
    let mut rows = statement.query([project.to_string()])?;
    while let Some(row) = rows.next()? {
        let text: String = row.get(0)?;
        let stage: Option<String> = row.get(1)?;
        let descriptor: WorkspaceDescriptor = decode(&text)?;
        let owns = match (descriptor.backing, stage.as_deref()) {
            (_, Some("retired")) => {
                !matches!(descriptor.lifecycle, WorkspaceLifecycle::Archived { .. })
            }
            (_, Some(_)) | (WorkspaceBacking::Managed { .. }, None) => true,
            (WorkspaceBacking::Local, None) => matches!(
                descriptor.lifecycle,
                WorkspaceLifecycle::Archiving { .. } | WorkspaceLifecycle::Restoring { .. }
            ),
        };
        if owns {
            return Ok(Some(descriptor.id));
        }
    }
    let found: Option<String> = connection
        .query_row(
            "SELECT id FROM workspace_registry_reservations
             WHERE stage!='retired' AND json_extract(intent,'$.project')=?1 ORDER BY id LIMIT 1",
            [project.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    found.map(parse).transpose()
}

/// Retired receipts whose backing was removed do not make a live claim.
/// Exclusion is only used after the same workspace proves its archive token.
pub(crate) fn worktree(
    connection: &Connection,
    worktree: WorktreeId,
    excluding: Option<WorkspaceId>,
) -> Result<Option<WorkspaceId>, StoreError> {
    let found: Option<String> = connection
        .query_row(
            "SELECT id FROM (
               SELECT w.id FROM workspace_registry w
               LEFT JOIN workspace_registry_reservations r ON r.id=w.id
               WHERE json_extract(w.descriptor,'$.backing.kind')='managed'
               AND json_extract(w.descriptor,'$.backing.worktree')=?1
               AND (json_extract(w.descriptor,'$.lifecycle.kind')!='archived' OR r.stage IS NOT 'retired')
               UNION
               SELECT id FROM workspace_registry_reservations WHERE worktree=?1 AND stage!='retired'
             ) WHERE ?2 IS NULL OR id!=?2 ORDER BY id LIMIT 1",
            params![worktree.to_string(), excluding.map(|id| id.to_string())],
            |row| row.get(0),
        )
        .optional()?;
    found.map(parse).transpose()
}

fn parse(text: String) -> Result<WorkspaceId, StoreError> {
    text.parse()
        .map_err(|source| StoreError::CorruptId { text, source })
}
