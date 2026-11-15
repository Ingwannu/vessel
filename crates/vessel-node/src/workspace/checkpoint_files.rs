//! Opened mechanism for the controller's retained filesystem worker, without SQL.
use super::Workspaces;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{
    CaptureKey, CheckpointError, FileImage, ObservedSnapshot, PhysicalRoot, RestoreId, StagedPaths,
};
use vessel_store::checkpoint::Plan;
use vessel_workspace::{
    Git,
    checkpoint::{CaptureStorage, IgnoreDefaults, Restore, Storage, Tree},
};

#[derive(Debug)]
pub(crate) struct CheckpointFiles {
    path: PathBuf,
    physical: PhysicalRoot,
    capture: CaptureStorage,
    git: Git,
}
impl Workspaces {
    /// Call on the retained worker, after physical admission and durable quota.
    pub(crate) fn checkpoint_files(
        &self,
        key: CaptureKey,
        physical: PhysicalRoot,
    ) -> Result<CheckpointFiles, CheckpointError> {
        if self.stop.is_cancelled() {
            return Err(CheckpointError::Busy);
        }
        let path = {
            let state = self.state();
            let entry = state
                .worktrees
                .get(&key.worktree)
                .ok_or(CheckpointError::UnsupportedScope)?;
            if entry.running.is_some() {
                return Err(CheckpointError::Busy);
            }
            entry.info.path.clone()
        };
        let capture = Storage::open(&self.places.data)
            .map_err(|error| error.reason())?
            .open_capture(key)
            .map_err(|error| error.reason())?;
        Ok(CheckpointFiles {
            path,
            physical,
            capture,
            git: self.git.clone().ok_or(CheckpointError::UnsupportedScope)?,
        })
    }
}
impl CheckpointFiles {
    pub(crate) fn observe(
        &self,
        runtime: &tokio::runtime::Handle,
        defaults: &IgnoreDefaults,
        scope: &FileImage,
        cancel: &CancellationToken,
    ) -> Result<ObservedSnapshot, CheckpointError> {
        Tree::open(&self.path, self.physical)
            .map_err(|error| error.reason())?
            .observe_preview(runtime, &self.git, defaults, scope, cancel)
            .map_err(|error| error.reason())
    }
    pub(crate) fn prepare(
        &self,
        plan: &Plan,
        restore: RestoreId,
        cancel: &CancellationToken,
    ) -> Result<Restore, CheckpointError> {
        let archive = self.capture.archive().map_err(|error| error.reason())?;
        let stage = self
            .capture
            .create_restore(restore)
            .map_err(|error| error.reason())?;
        Tree::open(&self.path, self.physical)
            .map_err(|error| error.reason())?
            .prepare(&archive, stage.path(), &plan.current, &plan.desired, cancel)
            .map_err(|error| error.reason())
    }
    pub(crate) fn recovering(
        &self,
        plan: &Plan,
        restore: RestoreId,
        staged: &StagedPaths,
    ) -> Result<Restore, CheckpointError> {
        let stage = self
            .capture
            .open_restore(restore)
            .map_err(|error| error.reason())?;
        Tree::open(&self.path, self.physical)
            .map_err(|error| error.reason())?
            .recovering(stage.path(), &plan.current, &plan.desired, staged)
            .map_err(|error| error.reason())
    }
}
