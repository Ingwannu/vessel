//! No project mutation may precede the complete stable-object journal commit.
use super::{
    journal,
    paths::{self, Phase},
    records::{CheckpointStoreError as Error, Operation},
};
use rusqlite::{Connection, params};
use vessel_proto::checkpoint::{
    CheckpointError, FileState, ObservedPresence, RestoreOutcome, StagedPaths,
};

pub(super) fn commit(
    connection: &Connection,
    operation: &Operation,
    staged: &StagedPaths,
) -> Result<(), Error> {
    let current = journal::status(connection, operation.session, operation.id, operation.owner)?;
    if current.plan != operation.plan
        || !matches!(current.outcome, RestoreOutcome::InProgress { .. })
    {
        return Err(CheckpointError::RestoreIdConflict.into());
    }
    let (plan, _consumed) = super::plans::owned(connection, operation.plan, operation.owner)?;
    let entries = paths::list(connection, operation)?;
    if entries.len() != staged.entries().len() {
        return Err(CheckpointError::RecoveryPending.into());
    }
    for (entry, evidence) in entries.iter().zip(staged.entries()) {
        let original = plan
            .evidence
            .entries()
            .iter()
            .find(|observed| observed.path == entry.path)
            .and_then(|observed| match observed.presence {
                ObservedPresence::Absent => None,
                ObservedPresence::Present { object, .. } => Some(object),
            });
        if entry.path != evidence.path
            || original != evidence.before
            || matches!(entry.before, FileState::Present { .. }) != evidence.before.is_some()
            || matches!(entry.desired, FileState::Present { .. }) != evidence.desired.is_some()
        {
            return Err(CheckpointError::RecoveryPending.into());
        }
        if let Some(existing) = &entry.staged {
            if existing != evidence {
                return Err(CheckpointError::RestoreIdConflict.into());
            }
            continue;
        }
        if entry.phase != Phase::Pending {
            return Err(CheckpointError::RestoreIdConflict.into());
        }
        let changed = connection.execute(
            "UPDATE checkpoint_paths SET staged=?1 WHERE restore=?2 AND path=?3 AND phase='pending' AND staged IS NULL",
            params![serde_json::to_string(evidence)?, operation.id.to_string(), entry.path.to_string()],
        )?;
        if changed != 1 {
            return Err(CheckpointError::RestoreIdConflict.into());
        }
    }
    Ok(())
}
