//! Journal barriers surround actual per-path writes on one retained worker.
//!
//! [Decision Log]
//! - Purpose: a multi-file failure must report verified rollback or pinned recovery.
//! - Constraint: filesystem renames and SQL commits cannot share a transaction.
//! - Choice: persist the full stage evidence before first mutation; each verified
//!   apply advances its intent. Failure first fences recovery, then undoes only
//!   unchanged owned objects and verifies the complete original scope.
//! - Cost: external edits or uncertain storage preserve bytes and recovery debt.
use super::backend::reason;
use crate::workspace::CheckpointFiles;
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{CheckpointError, ObservedSnapshot, PathCount, RestoreOutcome};
use vessel_store::{
    Store,
    checkpoint::{CheckpointWrite, Operation, PathIntent, PathPhase, Plan},
};
use vessel_workspace::checkpoint::{IgnoreDefaults, RecoveryState, Restore, Rollback};

#[derive(Debug)]
pub(super) struct Execution<'a> {
    pub(super) runtime: &'a tokio::runtime::Handle,
    pub(super) store: &'a Store,
    pub(super) plan: &'a Plan,
    pub(super) operation: &'a Operation,
    pub(super) defaults: &'a IgnoreDefaults,
    pub(super) cancel: &'a CancellationToken,
}
impl Execution<'_> {
    fn write(&self, write: CheckpointWrite) -> Result<(), CheckpointError> {
        self.runtime
            .block_on(self.store.checkpoint_write(write))
            .map_err(reason)
    }
    pub(super) fn run(&self, files: &CheckpointFiles) -> Result<RestoreOutcome, CheckpointError> {
        let count = PathCount::new(self.plan.desired.changes(&self.plan.current).len())?;
        if count.get() == 0 {
            let verified = files.observe(
                self.runtime,
                self.defaults,
                self.plan.desired.image(),
                self.cancel,
            )?;
            self.verify_desired(&verified)?;
            self.write(CheckpointWrite::Outcome {
                operation: Box::new(self.operation.clone()),
                result: RestoreOutcome::Unchanged,
            })?;
            return Ok(RestoreOutcome::Unchanged);
        }
        let restore = match files.prepare(self.plan, self.operation.id, self.cancel) {
            Ok(restore) => restore,
            Err(failure) => return self.rollback(files, None, failure),
        };
        let apply = (|| {
            self.write(CheckpointWrite::Staged {
                operation: Box::new(self.operation.clone()),
                paths: restore.evidence().map_err(|error| error.reason())?,
            })?;
            let paths = self
                .runtime
                .block_on(self.store.checkpoint_paths(self.operation.clone()))
                .map_err(reason)?;
            for (index, intent) in paths.into_iter().enumerate() {
                restore
                    .apply(PathCount::new(index)?, self.cancel)
                    .map_err(|error| error.reason())?;
                self.write(CheckpointWrite::Advance {
                    operation: Box::new(self.operation.clone()),
                    path: intent.path,
                    from: PathPhase::Pending,
                    to: PathPhase::Applied,
                })?;
            }
            let verified = files.observe(
                self.runtime,
                self.defaults,
                self.plan.desired.image(),
                self.cancel,
            )?;
            self.verify_desired(&verified)?;
            let outcome = RestoreOutcome::Restored { changed: count };
            self.write(CheckpointWrite::Outcome {
                operation: Box::new(self.operation.clone()),
                result: outcome.clone(),
            })?;
            Ok(outcome)
        })();
        match apply {
            Ok(outcome) => Ok(outcome),
            Err(failure) => self.rollback(files, Some(&restore), failure),
        }
    }
    fn verify_desired(&self, observed: &ObservedSnapshot) -> Result<(), CheckpointError> {
        if self.plan.desired.matches(observed.image())
            && observed.evidence().physical() == self.plan.evidence.physical()
            && observed.evidence().root() == self.plan.evidence.root()
            && observed.evidence().ignore() == self.plan.evidence.ignore()
        {
            Ok(())
        } else {
            Err(CheckpointError::CaptureUnstable)
        }
    }
    pub(super) fn rollback(
        &self,
        files: &CheckpointFiles,
        restore: Option<&Restore>,
        failure: CheckpointError,
    ) -> Result<RestoreOutcome, CheckpointError> {
        // An outcome acknowledgement may have been lost. Never undo a durable
        // terminal success merely because its original caller saw an SQL error.
        let current = self
            .runtime
            .block_on(self.store.checkpoint_operation(
                self.operation.session,
                self.operation.id,
                self.operation.owner,
            ))
            .map_err(reason)?;
        match current.outcome {
            RestoreOutcome::InProgress { .. } => self.write(CheckpointWrite::Outcome {
                operation: Box::new(current.clone()),
                result: RestoreOutcome::RecoveryRequired {
                    restore: current.id,
                },
            })?,
            RestoreOutcome::RecoveryRequired { .. } => {}
            RestoreOutcome::Restored { .. }
            | RestoreOutcome::Unchanged
            | RestoreOutcome::RolledBack { .. } => return Ok(current.outcome),
        }
        let paths = self
            .runtime
            .block_on(self.store.checkpoint_paths(current.clone()))
            .map_err(reason)?;
        // Cleanup has its own uncancelled token. It may outlive the request
        // deadline, retaining the original worker/root instead of spawning one.
        let cleanup = CancellationToken::new();
        if let Some(restore) = restore {
            for (index, intent) in paths.iter().enumerate().rev() {
                let index = PathCount::new(index)?;
                match restore
                    .recovery_state(index, &cleanup)
                    .map_err(|error| error.reason())?
                {
                    RecoveryState::Original => {}
                    RecoveryState::Desired => {
                        if restore
                            .undo(index, &cleanup)
                            .map_err(|error| error.reason())?
                            != Rollback::Restored
                        {
                            self.conflict(&current, intent)?;
                            return Err(CheckpointError::RecoveryPending);
                        }
                    }
                    RecoveryState::Conflict => {
                        self.conflict(&current, intent)?;
                        return Err(CheckpointError::RecoveryPending);
                    }
                }
                if matches!(intent.phase, PathPhase::Pending | PathPhase::Applied) {
                    self.write(CheckpointWrite::Advance {
                        operation: Box::new(current.clone()),
                        path: intent.path.clone(),
                        from: intent.phase,
                        to: PathPhase::Restored,
                    })?;
                }
            }
        }
        let observed = files.observe(self.runtime, self.defaults, &self.plan.current, &cleanup)?;
        // The store rechecks exact whole-image/root/ignore and original stable
        // object identities; matching bytes from an external replacement refuse.
        self.write(CheckpointWrite::Recovered {
            operation: Box::new(current),
            observed,
            reason: failure,
        })?;
        Ok(RestoreOutcome::RolledBack { reason: failure })
    }
    fn conflict(&self, operation: &Operation, intent: &PathIntent) -> Result<(), CheckpointError> {
        if matches!(intent.phase, PathPhase::Pending | PathPhase::Applied) {
            self.write(CheckpointWrite::Advance {
                operation: Box::new(operation.clone()),
                path: intent.path.clone(),
                from: intent.phase,
                to: PathPhase::Conflict,
            })?;
        }
        Ok(())
    }
}
