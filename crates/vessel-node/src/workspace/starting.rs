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
    workspaces: Arc<Workspaces>,
    #[cfg(target_os = "linux")]
    lease: RootLease,
}
impl StartGuard {
    #[cfg(target_os = "linux")]
    pub(crate) const fn physical(&self) -> vessel_proto::checkpoint::PhysicalRoot {
        self.lease.root()
    }
    pub(crate) fn folder(&self) -> &Path {
        &self.folder
    }
    /// Check again in the retained startup task, never after dropping its guard.
    #[cfg(target_os = "linux")]
    pub(crate) async fn revalidate(&self) -> Result<(), WorkspaceError> {
        let directory = Arc::clone(&self.directory);
        let expected = self.lease.root();
        self.workspaces
            .root_metadata(move || {
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
                directory: Arc::new(opened.directory),
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
