//! One trusted node boundary for principal-bound metadata and admitted file jobs.
use super::{backend::reason, bridge::Context};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use vessel_proto::checkpoint::{
    CHECKPOINT_QUEUE, ChangePage, CheckpointError, CheckpointOwner, CheckpointPage,
    CheckpointRestoreStatus, ListCheckpoints, ReadCheckpointChanges, RestoreCheckpoint,
    RestoreOutcome,
};

#[derive(Debug, Clone)]
pub(crate) struct Controller {
    pub(super) context: Context,
    pub(super) jobs: Arc<Semaphore>,
    pub(super) held: Arc<Mutex<Vec<Arc<super::workers::WorkerLease>>>>,
}
impl Controller {
    pub(super) fn new(context: Context) -> Self {
        Self {
            context,
            jobs: Arc::new(Semaphore::new(CHECKPOINT_QUEUE)),
            held: Arc::default(),
        }
    }
    pub(crate) async fn list(
        &self,
        request: ListCheckpoints,
    ) -> Result<CheckpointPage, CheckpointError> {
        self.context
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
        self.context
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
        self.context
            .sessions
            .get(request.session)
            .await
            .map_err(|_missing| CheckpointError::NotFound)?;
        self.context
            .store
            .checkpoint_operation(request.session, request.restore, owner)
            .await
            .map(|operation| operation.outcome)
            .map_err(reason)
    }
    pub(crate) fn restore(
        &self,
        _owner: CheckpointOwner,
        _request: RestoreCheckpoint,
    ) -> Result<RestoreOutcome, CheckpointError> {
        let _request = Arc::clone(&self.jobs)
            .try_acquire_owned()
            .map_err(|_busy| CheckpointError::Busy)?;
        Err(CheckpointError::FeatureUnavailable)
    }
}
