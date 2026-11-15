//! CAS writes and lifecycle admission share the writer's existing savepoint.

use super::{RegistryRecord, RegistryReservation, RegistryWrite, ReservationStage, read};
use crate::{
    StoreError,
    codec::{encode, time_to_sql},
};
use rusqlite::{Connection, params};
use vessel_proto::WorkspaceId;
use vessel_proto::registry::{RegistryRevision, WorkspaceLifecycle, WorkspaceResource};

/// Label/catalog mutations call this in their same transaction.
pub(crate) fn bump_revision(connection: &Connection) -> Result<(), StoreError> {
    let changed=connection.execute("UPDATE workspace_registry_meta SET revision=revision+1 WHERE singleton=1 AND revision < ?1",[i64::MAX])?;
    if changed == 0 {
        return Err(StoreError::RegistryRevisionExhausted);
    }
    Ok(())
}

/// Every member/label write checks lifecycle within its writer transaction.
pub(crate) fn require_active(connection: &Connection, id: WorkspaceId) -> Result<(), StoreError> {
    let record = read::workspace(connection, id)?.ok_or(StoreError::RegistryConflict)?;
    match record.descriptor.lifecycle {
        WorkspaceLifecycle::Active => Ok(()),
        WorkspaceLifecycle::Archiving { .. } | WorkspaceLifecycle::Archived { .. } => {
            Err(StoreError::RegistryConflict)
        }
    }
}

fn revision(value: RegistryRevision) -> Result<i64, StoreError> {
    i64::try_from(value.get()).map_err(|_| StoreError::RegistryRevisionExhausted)
}

pub(crate) fn apply(connection: &Connection, mutation: &RegistryWrite) -> Result<(), StoreError> {
    match mutation {
        RegistryWrite::RecoverCreating => {
            let changed = connection.execute("UPDATE workspace_registry_reservations SET stage='uncertain' WHERE stage='creating'",[])?;
            if changed == 0 {
                return Ok(());
            }
        }
        RegistryWrite::Workspace { record, expected } => put(connection, record, *expected)?,
        RegistryWrite::Membership {
            workspace,
            resource,
            expected,
        } => membership(connection, *workspace, *resource, *expected)?,
        RegistryWrite::Reserve(reservation) => reserve(connection, reservation)?,
        RegistryWrite::Stage {
            workspace,
            expected,
            stage: next,
        } => stage(connection, *workspace, *expected, *next)?,
        RegistryWrite::Completed(record) => {
            put(connection, record, None)?;
            stage(
                connection,
                record.descriptor.id,
                ReservationStage::Creating,
                ReservationStage::Ready,
            )?;
        }
    }
    bump_revision(connection)
}

fn put(
    connection: &Connection,
    record: &RegistryRecord,
    expected: Option<RegistryRevision>,
) -> Result<(), StoreError> {
    let descriptor = &record.descriptor;
    let next = match expected {
        None => RegistryRevision::ZERO,
        Some(previous) => previous
            .next()
            .ok_or(StoreError::RegistryRevisionExhausted)?,
    };
    if descriptor.revision != next {
        return Err(StoreError::RegistryConflict);
    }
    if let Some(previous) = expected {
        let old =
            read::workspace(connection, descriptor.id)?.ok_or(StoreError::RegistryConflict)?;
        if old.descriptor.revision != previous
            || old.descriptor.project != descriptor.project
            || old.descriptor.path != descriptor.path
            || old.descriptor.backing != descriptor.backing
            || old.descriptor.created_at != descriptor.created_at
        {
            return Err(StoreError::RegistryConflict);
        }
    }
    let encoded = encode(descriptor)?;
    let recovery = record.recovery.as_ref().map(encode).transpose()?;
    let changed=match expected {
        None=>connection.execute("INSERT INTO workspace_registry(id,project,created_at,revision,descriptor,recovery) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO NOTHING",params![descriptor.id.to_string(),descriptor.project.to_string(),time_to_sql(descriptor.created_at)?,revision(next)?,encoded,recovery])?,
        Some(previous)=>connection.execute("UPDATE workspace_registry SET revision=?2,descriptor=?3,recovery=?4 WHERE id=?1 AND revision=?5",params![descriptor.id.to_string(),revision(next)?,encoded,recovery,revision(previous)?])?,
    };
    if changed == 0 {
        return Err(StoreError::RegistryConflict);
    }
    Ok(())
}

fn membership(
    connection: &Connection,
    workspace: WorkspaceId,
    resource: WorkspaceResource,
    expected: RegistryRevision,
) -> Result<(), StoreError> {
    require_active(connection, workspace)?;
    let mut record = read::workspace(connection, workspace)?.ok_or(StoreError::RegistryConflict)?;
    if record.descriptor.revision != expected {
        return Err(StoreError::RegistryConflict);
    }
    let (kind, id) = match resource {
        WorkspaceResource::Session { session } => ("session", session.to_string()),
        WorkspaceResource::Terminal { terminal } => ("terminal", terminal.to_string()),
    };
    connection.execute("INSERT INTO workspace_registry_resources(kind,resource,workspace) VALUES (?1,?2,?3) ON CONFLICT(kind,resource) DO NOTHING",params![kind,id,workspace.to_string()])?;
    let owner: String = connection.query_row(
        "SELECT workspace FROM workspace_registry_resources WHERE kind=?1 AND resource=?2",
        params![kind, id],
        |row| row.get(0),
    )?;
    if owner != workspace.to_string() {
        return Err(StoreError::RegistryConflict);
    }
    // Admission changes the descriptor revision too: archive cannot drain an older member set.
    record.descriptor.revision = expected
        .next()
        .ok_or(StoreError::RegistryRevisionExhausted)?;
    put(connection, &record, Some(expected))
}

fn reserve(connection: &Connection, reservation: &RegistryReservation) -> Result<(), StoreError> {
    connection.execute("INSERT INTO workspace_registry_reservations(id,intent,worktree,stage) VALUES (?1,?2,?3,?4) ON CONFLICT(id) DO NOTHING",params![reservation.workspace.to_string(),encode(&reservation.intent)?,reservation.worktree.map(|id|id.to_string()),reservation.stage.sql()])?;
    let found = read::reservation(connection, reservation.workspace)?
        .ok_or(StoreError::RegistryConflict)?;
    if found.intent != reservation.intent || found.worktree != reservation.worktree {
        return Err(StoreError::RegistryConflict);
    }
    Ok(())
}

fn stage(
    connection: &Connection,
    workspace: WorkspaceId,
    expected: ReservationStage,
    next: ReservationStage,
) -> Result<(), StoreError> {
    if next == ReservationStage::Ready && read::workspace(connection, workspace)?.is_none() {
        return Err(StoreError::RegistryConflict);
    }
    let changed = connection.execute(
        "UPDATE workspace_registry_reservations SET stage=?2 WHERE id=?1 AND stage=?3",
        params![workspace.to_string(), next.sql(), expected.sql()],
    )?;
    if changed == 0 {
        return Err(StoreError::RegistryConflict);
    }
    Ok(())
}
