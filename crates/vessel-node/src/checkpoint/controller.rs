//! One trusted node boundary for principal-bound metadata and admitted file jobs.
use super::{backend::reason, bridge::Context};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use vessel_proto::checkpoint::{
    CHECKPOINT_QUEUE, ChangePage, CheckpointError, CheckpointOwner, CheckpointPage,
    CheckpointRestoreStatus, ListCheckpoints, ReadCheckpointChanges, RestoreOutcome,
};

#[derive(Debug, Clone)]
pub(crate) struct Controller {
    pub(super) context: Context,
    pub(super) jobs: Arc<Semaphore>,
    pub(super) held: Arc<Mutex<Vec<Arc<super::workers::WorkerLease>>>>,
    pub(super) terminals: TerminalAdmission,
}
impl Controller {
    pub(super) fn new(context: Context) -> Self {
        Self {
            context,
            jobs: Arc::new(Semaphore::new(CHECKPOINT_QUEUE)),
            held: Arc::default(),
            terminals: TerminalAdmission::Unavailable,
        }
    }
    pub(crate) async fn list(
        &self,
        request: ListCheckpoints,
    ) -> Result<CheckpointPage, CheckpointError> {
        let _visible = self
            .context
            .sessions
            .get(request.session)
            .await
            .map_err(|_missing| CheckpointError::NotFound)?;
        self.context
            .store
            .checkpoint_page(request.session, request.after)
            .await
            .map_err(reason)
    }
    pub(crate) async fn changes(
        &self,
        owner: CheckpointOwner,
        request: ReadCheckpointChanges,
    ) -> Result<ChangePage, CheckpointError> {
        let (plan, _consumed) = self
            .context
            .store
            .checkpoint_plan(request.plan, owner)
            .await
            .map_err(reason)?;
        let _visible = self
            .context
            .sessions
            .get(plan.key.session)
            .await
            .map_err(|_missing| CheckpointError::NotFound)?;
        self.context
            .store
            .checkpoint_changes(request.plan, owner, request.after, chrono::Utc::now())
            .await
            .map_err(reason)
    }
    pub(crate) async fn status(
        &self,
        owner: CheckpointOwner,
        request: CheckpointRestoreStatus,
    ) -> Result<RestoreOutcome, CheckpointError> {
        let _visible = self
            .context
            .sessions
            .get(request.session)
            .await
            .map_err(|_missing| CheckpointError::NotFound)?;
        let operation = self
            .context
            .store
            .checkpoint_operation(request.session, request.restore, owner)
            .await
            .map_err(reason)?;
        // File success alone cannot tell a reconnecting client that its result
        // was published. A retry finishes metadata without reapplying files.
        if matches!(operation.outcome, RestoreOutcome::Restored { .. })
            && self
                .context
                .store
                .checkpoint_event_seq(operation.clone())
                .await
                .map_err(reason)?
                .is_none()
        {
            return Ok(RestoreOutcome::InProgress {
                restore: operation.id,
            });
        }
        Ok(operation.outcome)
    }
}

/// Legacy absence/ended flags cannot certify forgotten runner cleanup debt.
#[derive(Debug, Clone, Copy)]
pub(super) enum TerminalAdmission {
    Unavailable,
    #[cfg(test)]
    NoTerminalFixture,
}
impl Controller {
    pub(super) fn terminal_quiescence(&self) -> Result<(), CheckpointError> {
        match self.terminals {
            TerminalAdmission::Unavailable => Err(CheckpointError::Busy),
            #[cfg(test)]
            TerminalAdmission::NoTerminalFixture => Ok(()),
        }
    }
    #[cfg(test)]
    pub(crate) fn no_terminal_fixture(mut self) -> Self {
        self.terminals = TerminalAdmission::NoTerminalFixture;
        self
    }
}
