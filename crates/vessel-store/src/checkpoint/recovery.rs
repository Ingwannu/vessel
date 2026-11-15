//! Restart never treats an interrupted filesystem operation as committed success.
use super::records::CheckpointStoreError as Error;
use rusqlite::{Connection, params};
use vessel_proto::checkpoint::{CHECKPOINT_PAGE_ENTRIES, RestoreId, RestoreOutcome};

/// Advance a bounded batch; caller repeats while it receives a full batch.
pub(super) fn pending(connection: &Connection) -> Result<usize, Error> {
    let mut rows = connection.prepare(
        "SELECT id FROM checkpoint_restores WHERE phase='prepared' ORDER BY id LIMIT ?1",
    )?;
    let ids = rows
        .query_map(
            [i64::try_from(CHECKPOINT_PAGE_ENTRIES)
                .map_err(|_large| vessel_proto::checkpoint::CheckpointError::TooManyPaths)?],
            |row| row.get::<_, String>(0),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    for raw in &ids {
        let restore = raw.parse::<RestoreId>().map_err(Error::Id)?;
        let result = serde_json::to_string(&RestoreOutcome::RecoveryRequired { restore })?;
        let _changed = connection.execute("UPDATE checkpoint_restores SET phase='recovery',outcome=?1 WHERE id=?2 AND phase='prepared'",params![result,raw])?;
    }
    Ok(ids.len())
}

/// Caller holds root exclusion while verifying and committing the whole before image.
/// Path conflicts remain pinned unless their original object and bytes are present.
pub(super) fn resolved(
    connection: &Connection,
    operation: &super::records::Operation,
    observed: &vessel_proto::checkpoint::ObservedSnapshot,
    reason: vessel_proto::checkpoint::CheckpointError,
) -> Result<(), Error> {
    use vessel_proto::checkpoint::CheckpointError;
    let current =
        super::journal::status(connection, operation.session, operation.id, operation.owner)?;
    let result = RestoreOutcome::RolledBack { reason };
    if current.plan != operation.plan {
        return Err(CheckpointError::RestoreIdConflict.into());
    }
    if current.outcome == result {
        return Ok(());
    }
    if !matches!(current.outcome,RestoreOutcome::RecoveryRequired {restore} if restore==current.id)
    {
        return Err(CheckpointError::RestoreIdConflict.into());
    }
    let (plan, _consumed) = super::plans::owned(connection, current.plan, current.owner)?;
    verify_before(&current, &plan, observed)?;
    let paths = super::paths::list(connection, &current)?;
    if paths.len() != plan.desired.changes(&current.before).len() {
        return Err(CheckpointError::RecoveryPending.into());
    }
    let _changed = connection.execute(
        "UPDATE checkpoint_paths SET phase='restored' WHERE restore=?1",
        [current.id.to_string()],
    )?;
    let changed=connection.execute("UPDATE checkpoint_restores SET phase='complete',outcome=?1 WHERE id=?2 AND phase='recovery' AND outcome=?3",
        params![serde_json::to_string(&result)?,current.id.to_string(),serde_json::to_string(&current.outcome)?])?;
    if changed != 1 {
        return Err(CheckpointError::RecoveryPending.into());
    }
    Ok(())
}

fn verify_before(
    current: &super::records::Operation,
    plan: &super::records::Plan,
    observed: &vessel_proto::checkpoint::ObservedSnapshot,
) -> Result<(), Error> {
    use vessel_proto::checkpoint::{CheckpointError, ObservedPresence};
    if observed.image() != &current.before
        || observed.evidence().physical() != plan.evidence.physical()
        || observed.evidence().root() != plan.evidence.root()
        || observed.evidence().ignore() != plan.evidence.ignore()
        || observed.evidence().entries().len() != plan.evidence.entries().len()
    {
        return Err(CheckpointError::RecoveryPending.into());
    }
    for (before, after) in plan
        .evidence
        .entries()
        .iter()
        .zip(observed.evidence().entries())
    {
        let original = match (before.presence, after.presence) {
            (ObservedPresence::Absent, ObservedPresence::Absent) => true,
            (
                ObservedPresence::Present { object: before, .. },
                ObservedPresence::Present { object: after, .. },
            ) => before == after,
            (ObservedPresence::Absent, ObservedPresence::Present { .. })
            | (ObservedPresence::Present { .. }, ObservedPresence::Absent) => false,
        };
        if before.path != after.path || !original {
            return Err(CheckpointError::RecoveryPending.into());
        }
    }
    Ok(())
}
