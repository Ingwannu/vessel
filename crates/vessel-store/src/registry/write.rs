//! CAS writes and lifecycle admission share the writer's existing savepoint.

use super::{
    RegistryChange, RegistryRecord, RegistryReservation, RegistryWrite, ReservationStage, read,
};
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
    let record = read::workspace(connection, id)?.ok_or(StoreError::RegistryMissing(id))?;
    match record.descriptor.lifecycle {
        WorkspaceLifecycle::Active => Ok(()),
        WorkspaceLifecycle::Archiving { .. }
        | WorkspaceLifecycle::Restoring { .. }
        | WorkspaceLifecycle::Archived { .. } => Err(StoreError::RegistryInactive(id)),
    }
}

pub(super) fn revision(value: RegistryRevision) -> Result<i64, StoreError> {
    i64::try_from(value.get()).map_err(StoreError::RegistryRevisionRange)
}

pub(crate) fn apply(
    connection: &Connection,
    mutation: &RegistryWrite,
) -> Result<RegistryChange, StoreError> {
    match mutation {
        RegistryWrite::Restart { previous, expected } => {
            super::leases::restart(connection, previous, *expected)?;
        }
        RegistryWrite::Release(request) => super::automation::release(connection, request)?,
        RegistryWrite::Claim {
            workspace,
            resource,
            expected,
        } => super::leases::claim(connection, *workspace, *resource, *expected)?,
        RegistryWrite::Settle { lease, settled } => {
            super::leases::settle(connection, lease, *settled)?;
        }
        RegistryWrite::RecoverCreating => {
            let changed = connection.execute("UPDATE workspace_registry_reservations SET stage='uncertain' WHERE stage='creating'",[])?;
            let starts = connection.execute(
                "UPDATE workspace_registry_resources SET stage='uncertain' WHERE stage='pending'",
                [],
            )?;
            if changed == 0 && starts == 0 {
                return Ok(RegistryChange::Unchanged);
            }
        }
        RegistryWrite::Workspace { record, expected } => put(connection, record, *expected)?,
        RegistryWrite::Membership {
            workspace,
            resource,
            expected,
        } => membership(connection, *workspace, *resource, *expected)?,
        RegistryWrite::Reserve(reservation) => {
            if reserve(connection, reservation)? == RegistryChange::Unchanged {
                return Ok(RegistryChange::Unchanged);
            }
        }
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
    bump_revision(connection)?;
    Ok(RegistryChange::Changed)
}

pub(super) fn put(
    connection: &Connection,
    record: &RegistryRecord,
    expected: Option<RegistryRevision>,
) -> Result<(), StoreError> {
    let descriptor = &record.descriptor;
    super::capacity::workspace(connection, descriptor.id)?;
    if matches!(descriptor.lifecycle, WorkspaceLifecycle::Archived { .. }) {
        let unsettled:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM workspace_registry_resources WHERE workspace=?1 AND stage IN ('pending','uncertain'))",[descriptor.id.to_string()],|row|row.get(0))?;
        if unsettled {
            return Err(StoreError::RegistryConflict);
        }
    }
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
    super::capacity::bytes(&encoded, super::MAX_DESCRIPTOR_BYTES)?;
    let recovery = record.recovery.as_ref().map(encode).transpose()?;
    if let Some(text) = &recovery {
        super::capacity::bytes(text, super::MAX_RECOVERY_BYTES)?;
    }
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
    super::capacity::resource(connection, resource)?;
    let (kind, id) = super::resource_key(resource);
    let _inserted = connection.execute("INSERT INTO workspace_registry_resources(kind,resource,workspace) VALUES (?1,?2,?3) ON CONFLICT(kind,resource) DO NOTHING",params![kind,id,workspace.to_string()])?;
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

fn reserve(
    connection: &Connection,
    reservation: &RegistryReservation,
) -> Result<RegistryChange, StoreError> {
    super::capacity::workspace(connection, reservation.workspace)?;
    let intent = encode(&reservation.intent)?;
    super::capacity::bytes(&intent, super::MAX_INTENT_BYTES)?;
    let inserted = connection.execute("INSERT INTO workspace_registry_reservations(id,intent,worktree,stage) VALUES (?1,?2,?3,?4) ON CONFLICT(id) DO NOTHING",params![reservation.workspace.to_string(),intent,reservation.worktree.map(|id|id.to_string()),reservation.stage.sql()])?;
    let found = read::reservation(connection, reservation.workspace)?
        .ok_or(StoreError::RegistryConflict)?;
    if found.intent != reservation.intent || found.worktree != reservation.worktree {
        return Err(StoreError::RegistryConflict);
    }
    Ok(if inserted == 0 {
        RegistryChange::Unchanged
    } else {
        RegistryChange::Changed
    })
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
