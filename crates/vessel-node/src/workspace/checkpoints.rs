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
    pub(crate) fn verify_checkpoint_archive(
        &self,
        key: CaptureKey,
        image: &vessel_proto::checkpoint::FileImage,
        cancel: &CancellationToken,
    ) -> Result<(), CheckpointError> {
        let storage = Storage::open(&self.places.data).map_err(|error| error.reason())?;
        let capture = storage.open_capture(key).map_err(|error| error.reason())?;
        capture
            .archive()
            .map_err(|error| error.reason())?
            .verify_image(image, cancel)
            .map_err(|error| error.reason())
    }
    pub(crate) fn clean_checkpoint_storage(
        &self,
        worktree: vessel_proto::WorktreeId,
        capture: vessel_proto::checkpoint::CaptureId,
    ) -> Result<(), CheckpointError> {
        let storage = Storage::open(&self.places.data).map_err(|error| error.reason())?;
        crate::checkpoint::backend::clean(&storage, worktree, capture)
    }
    /// Only after the worker returns and its durable candidate is invalidated.
    pub(crate) fn clean_checkpoint(&self, key: CaptureKey) -> Result<(), CheckpointError> {
        self.clean_checkpoint_storage(key.worktree, key.capture)
    }
}

/// The caller owns the root lease and one node-wide worker through this actual I/O.
#[derive(Debug)]
pub(crate) struct PreviewRead {
    pub(crate) key: CaptureKey,
    pub(crate) physical: PhysicalRoot,
    pub(crate) selected: ObservedSnapshot,
    pub(crate) plan: vessel_proto::checkpoint::RestorePlanId,
}
impl Workspaces {
    pub(crate) fn preview_checkpoint(
        &self,
        runtime: &tokio::runtime::Handle,
        read: PreviewRead,
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
                .get(&read.key.worktree)
                .ok_or(CheckpointError::UnsupportedScope)?;
            if entry.running.is_some() {
                return Err(CheckpointError::Busy);
            }
            entry.info.path.clone()
        };
        if read.selected.evidence().physical() != read.physical {
            return Err(CheckpointError::StaleCapture);
        }
        let git = self.git.as_ref().ok_or(CheckpointError::UnsupportedScope)?;
        let capture = Storage::open(&self.places.data)
            .map_err(|error| error.reason())?
            .open_capture(read.key)
            .map_err(|error| error.reason())?;
        capture
            .archive()
            .map_err(|error| error.reason())?
            .verify_image(read.selected.image(), cancel)
            .map_err(|error| error.reason())?;
        let archive = capture
            .create_preview(read.plan)
            .map_err(|error| error.reason())?;
        let tree = Tree::open(&path, read.physical).map_err(|error| error.reason())?;
        let current = tree
            .capture_preview(
                runtime,
                git,
                &archive,
                defaults,
                read.selected.image(),
                cancel,
            )
            .map_err(|error| error.reason())?;
        if current.evidence().ignore() != read.selected.evidence().ignore() {
            return Err(CheckpointError::StaleCapture);
        }
        Ok(current)
    }
    pub(crate) fn clean_checkpoint_preview(
        &self,
        key: CaptureKey,
        plan: vessel_proto::checkpoint::RestorePlanId,
    ) -> Result<(), CheckpointError> {
        let capture = Storage::open(&self.places.data)
            .map_err(|error| error.reason())?
            .open_capture(key)
            .map_err(|error| error.reason())?;
        loop {
            match capture
                .clean_preview(plan)
                .map_err(|error| error.reason())?
            {
                vessel_workspace::checkpoint::Cleanup::Pending => {}
                vessel_workspace::checkpoint::Cleanup::Complete => return Ok(()),
            }
        }
    }
}
