//! Durable idempotency/admission. An expired plan never authorizes a different image.

use super::records::{CheckpointStoreError as Error, Operation, Plan};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::checkpoint::{
    CaptureKey, CheckpointError, CheckpointOwner, ObservedSnapshot, RestoreId, RestoreOutcome,
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
    super::quota::available(connection, reserved)?;
    let _changed = connection.execute("INSERT INTO checkpoint_captures(id,session,worktree,generation,boundary,state,reserved_bytes,created_at)
        VALUES (?1,?2,?3,?4,?5,'candidate',?6,?7)", params![key.capture.to_string(), key.session.to_string(), key.worktree.to_string(),
        encode(&key.generation)?, encode(&key.boundary)?, bytes, at.timestamp_millis()])?;
    Ok(())
}

pub(super) fn unavailable(
    connection: &Connection,
    key: CaptureKey,
    reason: CheckpointError,
) -> Result<(), Error> {
    let changed = connection.execute("UPDATE checkpoint_captures SET state='unavailable',image=NULL,evidence=NULL,reason=?6
        WHERE id=?1 AND session=?2 AND worktree=?3 AND generation=?4 AND boundary=?5 AND state='candidate'",
        params![key.capture.to_string(),key.session.to_string(),key.worktree.to_string(),encode(&key.generation)?,encode(&key.boundary)?,encode(&reason)?])?;
    if changed != 1 {
        return Err(CheckpointError::StaleCapture.into());
    }
    Ok(())
}

/// Persistence only; node publication must still validate the live capture lease.
pub(super) fn keep_candidate(
    connection: &Connection,
    key: CaptureKey,
    snapshot: &ObservedSnapshot,
) -> Result<(), Error> {
    let changed = connection.execute(
        "UPDATE checkpoint_captures SET image=?1,evidence=?2 WHERE id=?3 AND session=?4 AND worktree=?5
        AND generation=?6 AND boundary=?7 AND state='candidate'",
        params![
            encode(snapshot.image())?,
            encode(snapshot.evidence())?,
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
        "UPDATE checkpoint_captures SET state='unavailable',image=NULL,evidence=NULL,reason=?1 WHERE state='candidate'",
        [encode(&CheckpointError::StaleCapture)?],
    )?;
    Ok(())
}

pub(super) fn put_plan(
    connection: &Connection,
    plan: &Plan,
    at: DateTime<Utc>,
) -> Result<(), Error> {
    let _validated = ObservedSnapshot::new(plan.current.clone(), plan.evidence.clone())?;
    let lifetime =
        chrono::TimeDelta::from_std(vessel_proto::checkpoint::CHECKPOINT_PREVIEW_LIFETIME)?;
    let maximum = at
        .checked_add_signed(lifetime)
        .ok_or(CheckpointError::StalePreview)?;
    if plan.expires <= at || plan.expires > maximum {
        return Err(CheckpointError::StalePreview.into());
    }
    let active: i64 = connection.query_row(
        "SELECT count(*) FROM checkpoint_plans WHERE owner=?1 AND expires_at>?2 AND consumed_by IS NULL",
        params![encode(&plan.owner)?, at.timestamp_millis()],
        |row| row.get(0),
    )?;
    let active = usize::try_from(active).map_err(|_negative| CheckpointError::Busy)?;
    if active >= vessel_proto::checkpoint::CHECKPOINT_PREVIEWS_PER_OWNER {
        return Err(CheckpointError::Busy.into());
    }
    let ready: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM checkpoint_captures WHERE id=?1 AND session=?2 AND worktree=?3 AND generation=?4 AND boundary=?5 AND state='ready')",
        params![plan.key.capture.to_string(),plan.key.session.to_string(),plan.key.worktree.to_string(),encode(&plan.key.generation)?,encode(&plan.key.boundary)?],
        |row| row.get(0),
    )?;
    if !ready {
        return Err(CheckpointError::NotFound.into());
    }
    let selected: String = connection.query_row(
        "SELECT evidence FROM checkpoint_captures WHERE id=?1",
        [plan.key.capture.to_string()],
        |row| row.get(0),
    )?;
    let selected: vessel_proto::checkpoint::SnapshotEvidence = serde_json::from_str(&selected)?;
    if selected.physical() != plan.evidence.physical()
        || selected.ignore() != plan.evidence.ignore()
    {
        return Err(CheckpointError::StalePreview.into());
    }
    let _changed = connection.execute(
        "INSERT INTO checkpoint_plans(id,capture,owner,current_image,current_evidence,desired_image,expires_at)
        VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            plan.id.to_string(),
            plan.key.capture.to_string(),
            encode(&plan.owner)?,
            encode(&plan.current)?,
            encode(&plan.evidence)?,
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
    owner: CheckpointOwner,
) -> Result<Operation, Error> {
    let stored: Option<(String, String, String, String, String)> = connection
        .query_row(
            "SELECT plan,owner,outcome,before_image,session FROM checkpoint_restores WHERE id=?1",
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
    let (plan, recorded_owner, outcome, before, recorded_session) =
        stored.ok_or(CheckpointError::NotFound)?;
    let recorded_owner: CheckpointOwner = serde_json::from_str(&recorded_owner)?;
    if recorded_owner != owner || recorded_session != session.to_string() {
        return Err(CheckpointError::Forbidden.into());
    }
    Ok(Operation {
        session,
        id,
        owner,
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
    match status(connection, plan.key.session, restore, plan.owner) {
        Ok(existing) => {
            if existing.plan != plan.id {
                return Err(CheckpointError::RestoreIdConflict.into());
            }
            return Ok(existing);
        }
        Err(Error::Refused(CheckpointError::NotFound)) => {}
        Err(error) => return Err(error),
    }
    let (stored, consumed) = super::plans::owned(connection, plan.id, plan.owner)?;
    if stored.key.session != plan.key.session {
        return Err(CheckpointError::Forbidden.into());
    }
    if stored.expires <= at
        || consumed.is_some()
        || stored.key != plan.key
        || stored.evidence != plan.evidence
        || stored.current != plan.current
        || stored.desired != plan.desired
        || stored.expires.timestamp_millis() != plan.expires.timestamp_millis()
    {
        return Err(CheckpointError::StalePreview.into());
    }
    let outcome = RestoreOutcome::InProgress { restore };
    let _changed = connection.execute(
        "INSERT INTO checkpoint_restores(id,session,plan,owner,outcome,phase,before_image,created_at)
        VALUES (?1,?2,?3,?4,?5,'prepared',?6,?7)",
        params![
            restore.to_string(),
            plan.key.session.to_string(),
            plan.id.to_string(),
            encode(&plan.owner)?,
            encode(&outcome)?,
            encode(&stored.current)?,
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
        owner: plan.owner,
        outcome,
        before: plan.current.clone(),
    })
}

pub(super) fn outcome(
    connection: &Connection,
    operation: &Operation,
    result: &RestoreOutcome,
) -> Result<(), Error> {
    let current = status(connection, operation.session, operation.id, operation.owner)?;
    if current.plan != operation.plan {
        return Err(CheckpointError::RestoreIdConflict.into());
    }
    if current.outcome == *result {
        return Ok(());
    }
    super::paths::verify_terminal(connection, operation, result)?;
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
