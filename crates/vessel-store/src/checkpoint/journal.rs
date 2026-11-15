//! Durable idempotency/admission. An expired plan never authorizes a different image.

use super::records::{CheckpointStoreError as Error, Operation, Plan};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::{
    DeviceId,
    checkpoint::{CaptureKey, CheckpointError, FileImage, RestoreId, RestoreOutcome},
};

fn encode<T: serde::Serialize>(value: &T) -> Result<String, Error> {
    Ok(serde_json::to_string(value)?)
}

pub(super) fn candidate(
    connection: &Connection,
    key: CaptureKey,
    reserved: crate::ByteCount,
    at: DateTime<Utc>,
) -> Result<(), Error> {
    let bytes = i64::try_from(reserved.get()).map_err(|_large| CheckpointError::StorageFull)?;
    if reserved.get() > vessel_proto::checkpoint::CHECKPOINT_NODE_BYTES {
        return Err(CheckpointError::StorageFull.into());
    }
    let used: i64 = connection.query_row(
        "SELECT COALESCE(sum(reserved_bytes),0) FROM checkpoint_captures",
        [],
        |row| row.get(0),
    )?;
    let used = u64::try_from(used).map_err(|_negative| CheckpointError::StorageFull)?;
    let total = used
        .checked_add(reserved.get())
        .ok_or(CheckpointError::StorageFull)?;
    if total > vessel_proto::checkpoint::CHECKPOINT_NODE_BYTES {
        return Err(CheckpointError::StorageFull.into());
    }
    let _changed = connection.execute("INSERT INTO checkpoint_captures(id,session,worktree,generation,boundary,state,reserved_bytes,created_at)
        VALUES (?1,?2,?3,?4,?5,'candidate',?6,?7)", params![key.capture.to_string(), key.session.to_string(), key.worktree.to_string(),
        encode(&key.generation)?, encode(&key.boundary)?, bytes, at.timestamp_millis()])?;
    Ok(())
}

pub(super) fn unavailable(connection: &Connection, key: CaptureKey) -> Result<(), Error> {
    let changed = connection.execute("UPDATE checkpoint_captures SET state='unavailable',image=NULL
        WHERE id=?1 AND session=?2 AND worktree=?3 AND generation=?4 AND boundary=?5 AND state='candidate'",
        params![key.capture.to_string(),key.session.to_string(),key.worktree.to_string(),encode(&key.generation)?,encode(&key.boundary)?])?;
    if changed != 1 {
        return Err(CheckpointError::StaleCapture.into());
    }
    Ok(())
}

/// Persistence only; node publication must still validate the live capture lease.
pub(super) fn keep_candidate(
    connection: &Connection,
    key: CaptureKey,
    image: &FileImage,
) -> Result<(), Error> {
    let changed = connection.execute(
        "UPDATE checkpoint_captures SET image=?1 WHERE id=?2 AND session=?3 AND worktree=?4
        AND generation=?5 AND boundary=?6 AND state='candidate'",
        params![
            encode(image)?,
            key.capture.to_string(),
            key.session.to_string(),
            key.worktree.to_string(),
            encode(&key.generation)?,
            encode(&key.boundary)?
        ],
    )?;
    if changed != 1 {
        return Err(CheckpointError::StaleCapture.into());
    }
    Ok(())
}

/// Startup never promotes candidates; reservations remain until filesystem cleanup.
pub(super) fn invalidate_candidates(connection: &Connection) -> Result<(), Error> {
    let _changed = connection.execute(
        "UPDATE checkpoint_captures SET state='unavailable',image=NULL WHERE state='candidate'",
        [],
    )?;
    Ok(())
}

pub(super) fn put_plan(connection: &Connection, plan: &Plan) -> Result<(), Error> {
    let ready: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM checkpoint_captures WHERE id=?1 AND session=?2 AND worktree=?3 AND generation=?4 AND boundary=?5 AND state='ready')",
        params![plan.key.capture.to_string(),plan.key.session.to_string(),plan.key.worktree.to_string(),encode(&plan.key.generation)?,encode(&plan.key.boundary)?],
        |row| row.get(0),
    )?;
    if !ready {
        return Err(CheckpointError::NotFound.into());
    }
    let _changed = connection.execute(
        "INSERT INTO checkpoint_plans(id,capture,device,current_image,desired_image,expires_at)
        VALUES (?1,?2,?3,?4,?5,?6)",
        params![
            plan.id.to_string(),
            plan.key.capture.to_string(),
            plan.device.to_string(),
            encode(&plan.current)?,
            encode(&plan.desired)?,
            plan.expires.timestamp_millis()
        ],
    )?;
    Ok(())
}

pub(super) fn status(
    connection: &Connection,
    session: vessel_proto::SessionId,
    id: RestoreId,
    device: DeviceId,
) -> Result<Operation, Error> {
    let stored: Option<(String, String, String, String, String)> = connection
        .query_row(
            "SELECT plan,device,outcome,before_image,session FROM checkpoint_restores WHERE id=?1",
            [id.to_string()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()?;
    let (plan, owner, outcome, before, recorded_session) =
        stored.ok_or(CheckpointError::NotFound)?;
    let owner: DeviceId = serde_json::from_str(&encode(&owner)?)?;
    if owner != device || recorded_session != session.to_string() {
        return Err(CheckpointError::Forbidden.into());
    }
    Ok(Operation {
        session,
        id,
        device,
        plan: serde_json::from_str(&encode(&plan)?)?,
        outcome: serde_json::from_str(&outcome)?,
        before: serde_json::from_str(&before)?,
    })
}

/// Must run inside the existing writer's transaction/savepoint during wiring.
pub(super) fn admit(
    connection: &Connection,
    plan: &Plan,
    restore: RestoreId,
    at: DateTime<Utc>,
) -> Result<Operation, Error> {
    match status(connection, plan.key.session, restore, plan.device) {
        Ok(existing) => {
            if existing.plan != plan.id {
                return Err(CheckpointError::RestoreIdConflict.into());
            }
            return Ok(existing);
        }
        Err(Error::Refused(CheckpointError::NotFound)) => {}
        Err(error) => return Err(error),
    }
    let plan_data: Option<(String,String,String,i64,Option<String>)> = connection.query_row(
        "SELECT device,current_image,desired_image,expires_at,consumed_by FROM checkpoint_plans WHERE id=?1",
        [plan.id.to_string()], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?))).optional()?;
    let (owner, current, desired, expires, consumed) =
        plan_data.ok_or(CheckpointError::NotFound)?;
    if owner != plan.device.to_string() {
        return Err(CheckpointError::Forbidden.into());
    }
    if expires <= at.timestamp_millis() || consumed.is_some() {
        return Err(CheckpointError::StalePreview.into());
    }
    if serde_json::from_str::<FileImage>(&current)? != plan.current
        || serde_json::from_str::<vessel_proto::checkpoint::EffectiveDesiredImage>(&desired)?
            != plan.desired
    {
        return Err(CheckpointError::StalePreview.into());
    }
    let outcome = RestoreOutcome::InProgress { restore };
    let _changed = connection.execute(
        "INSERT INTO checkpoint_restores(id,session,plan,device,outcome,phase,before_image,created_at)
        VALUES (?1,?2,?3,?4,?5,'prepared',?6,?7)",
        params![
            restore.to_string(),
            plan.key.session.to_string(),
            plan.id.to_string(),
            plan.device.to_string(),
            encode(&outcome)?,
            current,
            at.timestamp_millis()
        ],
    )?;
    let _changed = connection.execute(
        "UPDATE checkpoint_plans SET consumed_by=?1 WHERE id=?2",
        params![restore.to_string(), plan.id.to_string()],
    )?;
    Ok(Operation {
        session: plan.key.session,
        id: restore,
        plan: plan.id,
        device: plan.device,
        outcome,
        before: plan.current.clone(),
    })
}

pub(super) fn outcome(
    connection: &Connection,
    operation: &Operation,
    result: &RestoreOutcome,
) -> Result<(), Error> {
    let current = status(
        connection,
        operation.session,
        operation.id,
        operation.device,
    )?;
    if current.plan != operation.plan {
        return Err(CheckpointError::RestoreIdConflict.into());
    }
    if current.outcome == *result {
        return Ok(());
    }
    match current.outcome {
        RestoreOutcome::InProgress { .. } => {}
        RestoreOutcome::Restored { .. }
        | RestoreOutcome::Unchanged
        | RestoreOutcome::RolledBack { .. }
        | RestoreOutcome::RecoveryRequired { .. } => {
            return Err(CheckpointError::RestoreIdConflict.into());
        }
    }
    let phase = match result {
        RestoreOutcome::InProgress { .. } => return Err(CheckpointError::RestoreIdConflict.into()),
        RestoreOutcome::RecoveryRequired { .. } => "recovery",
        RestoreOutcome::Restored { .. }
        | RestoreOutcome::Unchanged
        | RestoreOutcome::RolledBack { .. } => "complete",
    };
    let changed = connection.execute(
        "UPDATE checkpoint_restores SET outcome=?1,phase=?2 WHERE id=?3 AND outcome=?4 AND phase='prepared'",
        params![
            encode(result)?,
            phase,
            operation.id.to_string(),
            encode(&current.outcome)?
        ],
    )?;
    if changed != 1 {
        return Err(CheckpointError::RecoveryPending.into());
    }
    Ok(())
}
