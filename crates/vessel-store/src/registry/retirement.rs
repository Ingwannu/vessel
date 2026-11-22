//! Release only resolved registry capacity. Retired ids still carry the
//! provenance and replay fence until bounded history prunes them together.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::WorkspaceId;
use vessel_proto::registry::{WorkspaceBacking, WorkspaceDescriptor, WorkspaceLifecycle};

use super::{
    MAX_ARCHIVED_WORKSPACES, ProvisionRetirement, RegistryChange, ReservationStage, decode, read,
};
use crate::StoreError;
use crate::codec::{time_from_sql, time_to_sql};

pub(super) fn retire(
    connection: &Connection,
    workspace: WorkspaceId,
    evidence: ProvisionRetirement,
    at: DateTime<Utc>,
) -> Result<RegistryChange, StoreError> {
    let reservation =
        read::reservation(connection, workspace)?.ok_or(StoreError::RegistryMissing(workspace))?;
    if reservation.stage == ReservationStage::Retired {
        return Ok(RegistryChange::Unchanged);
    }
    let pinned: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schedule_run_bindings WHERE workspace=?1 AND stage NOT IN ('retiring_unpublished','retiring_archive'))",
        [workspace.to_string()],
        |row| row.get(0),
    )?;
    if pinned || !super::pending_starts(connection, workspace)?.is_empty() {
        return Err(StoreError::RegistryConflict);
    }
    let record = read::workspace(connection, workspace)?;
    match (evidence, record) {
        (ProvisionRetirement::Unpublished, None)
            if reservation.stage != ReservationStage::Ready => {}
        (ProvisionRetirement::CleanArchive, Some(record))
            if reservation.stage == ReservationStage::Ready
                && matches!(
                    record.descriptor.lifecycle,
                    WorkspaceLifecycle::Archived { .. }
                ) =>
        {
            let expected = match record.descriptor.backing {
                WorkspaceBacking::Local => None,
                WorkspaceBacking::Managed { worktree } => Some(worktree),
            };
            if reservation.worktree != expected {
                return Err(StoreError::RegistryConflict);
            }
        }
        _ => return Err(StoreError::RegistryConflict),
    }
    if let Some(worktree) = reservation.worktree {
        // The producer proves filesystem disposition; the Store also fences
        // a retained backing record, including the one a failed cleanup kept.
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM worktrees WHERE id=?1)",
            [worktree.to_string()],
            |row| row.get(0),
        )?;
        if exists {
            return Err(StoreError::RegistryConflict);
        }
    }
    let written = connection.execute(
        "UPDATE workspace_registry_reservations SET stage='retired',retired_at=?2 WHERE id=?1 AND stage=?3",
        params![workspace.to_string(), time_to_sql(at)?, reservation.stage.sql()],
    )?;
    if written != 1 {
        return Err(StoreError::RegistryConflict);
    }
    prune(connection)?;
    Ok(RegistryChange::Changed)
}

/// Both kinds of settled history share one limit; an unresolved binding pins
/// a workspace even if its schedule or bounded run history was deleted.
pub(super) fn prune(connection: &Connection) -> Result<(), StoreError> {
    let mut history = Vec::new();
    {
        let mut statement = connection.prepare_cached(
            "SELECT id,retired_at FROM workspace_registry_reservations r
             WHERE stage='retired' AND NOT EXISTS(SELECT 1 FROM schedule_run_bindings b WHERE b.workspace=r.id AND b.stage NOT IN ('retiring_unpublished','retiring_archive'))",
        )?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            history.push((time_from_sql(row.get(1)?), row.get::<_, String>(0)?));
        }
    }
    {
        let mut statement = connection.prepare_cached(
            "SELECT id,descriptor FROM workspace_registry w
             WHERE json_extract(descriptor,'$.lifecycle.kind')='archived'
             AND json_extract(descriptor,'$.backing.kind')='local'
             AND NOT EXISTS(SELECT 1 FROM workspace_registry_reservations r WHERE r.id=w.id)
             AND NOT EXISTS(SELECT 1 FROM schedule_run_bindings b WHERE b.workspace=w.id AND b.stage NOT IN ('retiring_unpublished','retiring_archive'))",
        )?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let raw: String = row.get(1)?;
            let descriptor: WorkspaceDescriptor = decode(&raw)?;
            if let WorkspaceLifecycle::Archived { at } = descriptor.lifecycle
                && descriptor.backing == WorkspaceBacking::Local
            {
                history.push((at, row.get(0)?));
            }
        }
    }
    let excess = history.len().saturating_sub(MAX_ARCHIVED_WORKSPACES);
    history.sort_unstable();
    for (_, id) in history.into_iter().take(excess) {
        let _descriptor =
            connection.execute("DELETE FROM workspace_registry WHERE id=?1", [&id])?;
        let _receipt = connection.execute(
            "DELETE FROM workspace_registry_reservations WHERE id=?1",
            [&id],
        )?;
    }
    Ok(())
}
