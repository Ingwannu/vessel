//! Per-path durable intent. Filesystem actions follow committed Pending rows.
use super::records::{CheckpointStoreError as Error, Operation};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use vessel_proto::checkpoint::{CheckpointError, CheckpointPath, FileState, RestoreOutcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Durable per-path progress; terminal outcomes verify the complete set.
pub enum Phase {
    /// Intent committed before filesystem mutation.
    Pending,
    /// Desired path installed and verified.
    Applied,
    /// Original path verified after rollback.
    Restored,
    /// External change prevents safe rollback.
    Conflict,
}
impl Phase {
    const fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Applied => "applied",
            Self::Restored => "restored",
            Self::Conflict => "conflict",
        }
    }
    const fn follows(self, previous: Self) -> bool {
        matches!(
            (previous, self),
            (
                Self::Pending,
                Self::Applied | Self::Restored | Self::Conflict
            ) | (Self::Applied, Self::Restored | Self::Conflict)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One immutable intent and its current durable progress.
pub struct Entry {
    /// Approved relative path.
    pub path: CheckpointPath,
    /// Pre-restore state.
    pub before: FileState,
    /// Frozen effective target.
    pub desired: FileState,
    /// Current durable progress.
    pub phase: Phase,
}

fn encode<T: Serialize>(value: &T) -> Result<String, Error> {
    Ok(serde_json::to_string(value)?)
}

/// Must share the writer transaction with durable restore admission, before writes.
pub(super) fn prepare(connection: &Connection, operation: &Operation) -> Result<(), Error> {
    let (plan, _consumed) = super::plans::owned(connection, operation.plan, operation.owner)?;
    let current =
        super::journal::status(connection, operation.session, operation.id, operation.owner)?;
    if current != *operation
        || plan.key.session != operation.session
        || !matches!(current.outcome, RestoreOutcome::InProgress { .. })
    {
        return Err(CheckpointError::RestoreIdConflict.into());
    }
    for change in plan.desired.changes(&operation.before) {
        let before = operation
            .before
            .get(&change.path)
            .cloned()
            .unwrap_or(FileState::Absent);
        let desired = plan
            .desired
            .image()
            .get(&change.path)
            .ok_or(CheckpointError::InvalidPath)?;
        let _changed = connection.execute(
            "INSERT INTO checkpoint_paths(restore,path,before_state,desired_state,phase) VALUES (?1,?2,?3,?4,'pending')",
            params![operation.id.to_string(),change.path.to_string(),encode(&before)?,encode(desired)?],
        )?;
    }
    Ok(())
}

/// Bound decoding before it can reach the worker or recovery coordinator.
pub(super) fn list(connection: &Connection, operation: &Operation) -> Result<Vec<Entry>, Error> {
    let _authorized =
        super::journal::status(connection, operation.session, operation.id, operation.owner)?;
    let mut statement = connection.prepare(
        "SELECT path,before_state,desired_state,phase FROM checkpoint_paths WHERE restore=?1 ORDER BY path LIMIT ?2",
    )?;
    let limit = vessel_proto::checkpoint::CHECKPOINT_PATHS.saturating_add(1);
    let raw = statement
        .query_map(
            params![
                operation.id.to_string(),
                i64::try_from(limit).map_err(|_large| CheckpointError::TooManyPaths)?
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let _count = vessel_proto::checkpoint::PathCount::new(raw.len())?;
    raw.into_iter()
        .map(|(path, before, desired, phase)| {
            Ok(Entry {
                path: serde_json::from_str(&encode(&path)?)?,
                before: serde_json::from_str(&before)?,
                desired: serde_json::from_str(&desired)?,
                phase: serde_json::from_str(&encode(&phase)?)?,
            })
        })
        .collect()
}

/// Compare-and-advance rejects a late callback after another recovery decision.
pub(super) fn advance(
    connection: &Connection,
    operation: &Operation,
    path: &CheckpointPath,
    previous: Phase,
    next: Phase,
) -> Result<(), Error> {
    let current =
        super::journal::status(connection, operation.session, operation.id, operation.owner)?;
    if current.plan != operation.plan
        || !matches!(
            current.outcome,
            RestoreOutcome::InProgress { .. } | RestoreOutcome::RecoveryRequired { .. }
        )
        || !next.follows(previous)
    {
        return Err(CheckpointError::RestoreIdConflict.into());
    }
    let changed = connection.execute(
        "UPDATE checkpoint_paths SET phase=?1 WHERE restore=?2 AND path=?3 AND phase=?4",
        params![
            next.label(),
            operation.id.to_string(),
            path.to_string(),
            previous.label()
        ],
    )?;
    if changed != 1 {
        return Err(CheckpointError::RestoreIdConflict.into());
    }
    Ok(())
}

/// A durable terminal outcome must agree with every approved path's journal.
pub(super) fn verify_terminal(
    connection: &Connection,
    operation: &Operation,
    result: &RestoreOutcome,
) -> Result<(), Error> {
    let (plan, _consumed) = super::plans::owned(connection, operation.plan, operation.owner)?;
    let entries = list(connection, operation)?;
    let changed = plan.desired.changes(&operation.before).len();
    let valid = match result {
        RestoreOutcome::Restored { changed: count } => {
            changed != 0
                && count.get() == changed
                && entries.len() == changed
                && entries.iter().all(|entry| entry.phase == Phase::Applied)
        }
        RestoreOutcome::Unchanged => changed == 0 && entries.is_empty(),
        RestoreOutcome::RolledBack { .. } => {
            entries.len() == changed && entries.iter().all(|entry| entry.phase == Phase::Restored)
        }
        RestoreOutcome::RecoveryRequired { restore } => *restore == operation.id,
        RestoreOutcome::InProgress { .. } => false,
    };
    if !valid {
        return Err(CheckpointError::RecoveryPending.into());
    }
    Ok(())
}
