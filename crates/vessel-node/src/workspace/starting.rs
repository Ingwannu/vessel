//! A retained start keeps admission until registration makes it visible to restore.
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
            let roots = self
                .state()
                .worktrees
                .values()
                .map(|entry| entry.info.path.clone())
                .collect::<Vec<_>>();
            let (folder, directory, root) = self
                .root_metadata(move || {
                    let directory = Directory::open(&folder)?;
                    let ancestry = directory.ancestry()?;
                    let mut known = BTreeSet::new();
                    for root in roots {
                        match Directory::open(&root) {
                            Ok(root) => {
                                let _inserted = known.insert(root.identity());
                            }
                            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                            Err(error) => return Err(error),
                        }
                    }
                    let root = ancestry
                        .nearest(&known)
                        .unwrap_or_else(|| directory.identity());
                    Ok::<_, std::io::Error>((folder, directory, root))
                })
                .await?;
            let deadline = tokio::time::Instant::now()
                .checked_add(ADMISSION_TIMEOUT)
                .ok_or(RootRefusal::TimedOut)?;
            let lease = self.exclusion.claim(root, deadline).await?;
            Ok(StartGuard {
                folder,
                directory: Arc::new(directory),
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
