//! Retained physical admission fences startup and owned lifecycle mutations.
use super::{WorkspaceError, Workspaces};
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(target_os = "linux")]
use {
    super::exclusion::{ADMISSION_TIMEOUT, RootLease, RootRefusal},
    std::collections::BTreeSet,
    vessel_workspace::physical::Directory,
};

#[derive(Debug)]
pub(crate) struct StartGuard {
    folder: PathBuf,
    #[cfg(target_os = "linux")]
    directory: Arc<Directory>,
    #[cfg(target_os = "linux")]
    root_directory: Arc<Directory>,
    #[cfg(target_os = "linux")]
    workspaces: Arc<Workspaces>,
    #[cfg(target_os = "linux")]
    lease: RootLease,
}
impl StartGuard {
    #[cfg(target_os = "linux")]
    pub(crate) const fn physical(&self) -> vessel_proto::checkpoint::PhysicalRoot {
        self.root_lease().root()
    }
    /// Borrow the exact opaque lease for a lifetime-bound quiescence proof.
    /// It cannot be cloned, recreated from a path, or outlive this guard.
    #[cfg(target_os = "linux")]
    pub(crate) const fn root_lease(&self) -> &RootLease {
        &self.lease
    }
    pub(crate) fn folder(&self) -> &Path {
        &self.folder
    }
    /// Check again in the retained startup task, never after dropping its guard.
    #[cfg(target_os = "linux")]
    pub(crate) async fn revalidate(&self) -> Result<(), WorkspaceError> {
        let directory = Arc::clone(&self.directory);
        let root_directory = Arc::clone(&self.root_directory);
        let expected = self.lease.root();
        self.workspaces
            .root_metadata(move || {
                root_directory.revalidate()?;
                directory.revalidate()?;
                if directory
                    .ancestry()?
                    .nearest(&BTreeSet::from([expected]))
                    .is_none()
                {
                    return Err(std::io::Error::other(
                        "start left its admitted physical root",
                    ));
                }
                Ok(())
            })
            .await?;
        Ok(())
    }
    /// A successful removal must leave no entry at its original name.
    pub(crate) async fn removed(&self, workspaces: &Workspaces) -> Result<(), WorkspaceError> {
        let folder = self.folder.clone();
        workspaces
            .root_metadata(move || match std::fs::symlink_metadata(folder) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
                Ok(_) => Err(std::io::Error::other(
                    "removed worktree still has a filesystem entry",
                )),
            })
            .await
    }
    #[cfg(not(target_os = "linux"))]
    pub(crate) fn revalidate(&self) -> std::future::Ready<Result<(), WorkspaceError>> {
        std::future::ready(if self.folder.is_absolute() {
            Ok(())
        } else {
            Err(WorkspaceError::Directory(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "start folder must be absolute",
            )))
        })
    }
}
impl Workspaces {
    pub(crate) async fn admit_start(
        self: &Arc<Self>,
        folder: &Path,
    ) -> Result<StartGuard, WorkspaceError> {
        if self.stop.is_cancelled() {
            return Err(WorkspaceError::Stopping);
        }
        let source = folder.to_path_buf();
        let folder = self
            .root_metadata(move || std::fs::canonicalize(source))
            .await?;
        #[cfg(target_os = "linux")]
        {
            let opened = self.opened_admission(&folder).await?;
            let deadline = tokio::time::Instant::now()
                .checked_add(ADMISSION_TIMEOUT)
                .ok_or(RootRefusal::TimedOut)?;
            let lease = self
                .exclusion
                .claim_registered(opened.root, opened.worktrees, deadline)
                .await?;
            Ok(StartGuard {
                folder,
                directory: opened.directory,
                root_directory: opened.root_directory,
                workspaces: Arc::clone(self),
                lease,
            })
        }
        #[cfg(not(target_os = "linux"))]
        Ok(StartGuard {
            folder: folder.to_path_buf(),
        })
    }
}

#[cfg(target_os = "linux")]
impl StartGuard {
    /// Pin before durable restore admission; uncertainty must not unlock writers.
    pub(crate) async fn pin_restore(
        &self,
        worktree: vessel_proto::WorktreeId,
        restore: vessel_proto::checkpoint::RestoreId,
    ) -> Result<(), vessel_proto::checkpoint::CheckpointError> {
        let deadline = tokio::time::Instant::now()
            .checked_add(ADMISSION_TIMEOUT)
            .ok_or(vessel_proto::checkpoint::CheckpointError::Busy)?;
        self.workspaces
            .exclusion
            .pin_recovery(&self.lease, worktree, restore, deadline)
            .await
            .map_err(|_refused| vessel_proto::checkpoint::CheckpointError::RecoveryPending)
    }
    /// Only after whole-image verification and the matching durable journal ack.
    pub(crate) async fn settle_restore(
        &self,
        restore: vessel_proto::checkpoint::RestoreId,
    ) -> Result<(), vessel_proto::checkpoint::CheckpointError> {
        let deadline = tokio::time::Instant::now()
            .checked_add(ADMISSION_TIMEOUT)
            .ok_or(vessel_proto::checkpoint::CheckpointError::Busy)?;
        self.workspaces
            .exclusion
            .resolve_recovery(&self.lease, restore, deadline)
            .await
            .map_err(|_refused| vessel_proto::checkpoint::CheckpointError::RecoveryPending)
    }
}
#[cfg(target_os = "linux")]
impl Workspaces {
    pub(crate) async fn checkpoint_recovery_admission(
        self: &Arc<Self>,
        worktree: vessel_proto::WorktreeId,
        physical: vessel_proto::checkpoint::PhysicalRoot,
        restore: vessel_proto::checkpoint::RestoreId,
    ) -> Result<StartGuard, vessel_proto::checkpoint::CheckpointError> {
        use vessel_proto::checkpoint::CheckpointError;
        let folder = self
            .state()
            .worktrees
            .get(&worktree)
            .map(|entry| entry.info.path.clone())
            .ok_or(CheckpointError::UnsupportedScope)?;
        let opened = self
            .opened_admission(&folder)
            .await
            .map_err(|_changed| CheckpointError::RecoveryPending)?;
        if opened.root != physical || !opened.worktrees.contains(&worktree) {
            return Err(CheckpointError::RecoveryPending);
        }
        let deadline = tokio::time::Instant::now()
            .checked_add(ADMISSION_TIMEOUT)
            .ok_or(CheckpointError::Busy)?;
        let lease = self
            .exclusion
            .claim_recovery(physical, restore, deadline)
            .await
            .map_err(|_refused| CheckpointError::RecoveryPending)?;
        Ok(StartGuard {
            folder,
            directory: opened.directory,
            root_directory: opened.root_directory,
            workspaces: Arc::clone(self),
            lease,
        })
    }
}

#[cfg(target_os = "linux")]
impl Workspaces {
    /// A completed retry uses the original pin identity, even if its image has
    /// since expired. This lookup grants no authority to mutate project files.
    pub(crate) async fn checkpoint_completion_admission(
        self: &Arc<Self>,
        restore: vessel_proto::checkpoint::RestoreId,
    ) -> Result<Option<StartGuard>, vessel_proto::checkpoint::CheckpointError> {
        use vessel_proto::checkpoint::CheckpointError;
        let deadline = tokio::time::Instant::now()
            .checked_add(ADMISSION_TIMEOUT)
            .ok_or(CheckpointError::Busy)?;
        let Some(pin) = self
            .exclusion
            .lookup_recovery(restore, deadline)
            .await
            .map_err(|_refused| CheckpointError::RecoveryPending)?
        else {
            return Ok(None);
        };
        self.checkpoint_recovery_admission(pin.worktree, pin.physical, restore)
            .await
            .map(Some)
    }
}
