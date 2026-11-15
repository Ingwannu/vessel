//! Managed identity lookup precedes the checkpoint worker's opened-root proof.
use super::Workspaces;
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{CaptureKey, CheckpointError, ObservedSnapshot, PhysicalRoot};
use vessel_workspace::checkpoint::{IgnoreDefaults, Storage, Tree};

impl Workspaces {
    pub(crate) async fn checkpoint_admission(
        self: &std::sync::Arc<Self>,
        worktree: vessel_proto::WorktreeId,
    ) -> Result<super::StartGuard, CheckpointError> {
        let path = self
            .state()
            .worktrees
            .get(&worktree)
            .map(|entry| entry.info.path.clone())
            .ok_or(CheckpointError::UnsupportedScope)?;
        self.admit_start(&path).await.map_err(|error| {
            if matches!(
                error,
                super::WorkspaceError::RootAdmission(
                    super::exclusion::RootRefusal::RecoveryPending
                )
            ) {
                CheckpointError::RecoveryPending
            } else {
                CheckpointError::Busy
            }
        })
    }
    /// Synchronous actual I/O: only the checkpoint runner's retained worker calls.
    pub(crate) fn capture_checkpoint(
        &self,
        runtime: &tokio::runtime::Handle,
        key: CaptureKey,
        physical: PhysicalRoot,
        defaults: &IgnoreDefaults,
        cancel: &CancellationToken,
    ) -> Result<ObservedSnapshot, CheckpointError> {
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
        let git = self.git.as_ref().ok_or(CheckpointError::UnsupportedScope)?;
        let storage = Storage::open(&self.places.data).map_err(|error| error.reason())?;
        let capture = storage
            .create_capture(key)
            .map_err(|error| error.reason())?;
        let archive = capture.archive().map_err(|error| error.reason())?;
        let tree = Tree::open(&path, physical).map_err(|error| error.reason())?;
        tree.capture(runtime, git, &archive, defaults, cancel)
            .map_err(|error| error.reason())
    }
    /// Only after the worker returns and its durable candidate is invalidated.
    pub(crate) fn clean_checkpoint(&self, key: CaptureKey) -> Result<(), CheckpointError> {
        let storage = Storage::open(&self.places.data).map_err(|error| error.reason())?;
        crate::checkpoint::backend::clean(&storage, key)
    }
}
