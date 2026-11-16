//! A guard pins both the cwd and the selected physical ancestor through real I/O.
use super::{
    WorkspaceError, Workspaces,
    exclusion::{ADMISSION_TIMEOUT, RootLease, RootRefusal},
};
use std::{collections::BTreeSet, path::Path, sync::Arc};
use vessel_workspace::physical::Directory;
#[derive(Debug)]
pub(crate) struct StartGuard {
    directory: Arc<Directory>,
    root_directory: Arc<Directory>,
    workspaces: Arc<Workspaces>,
    lease: RootLease,
}
impl StartGuard {
    /// Borrow the exact opaque lease; a future proof cannot outlive this guard.
    pub(crate) const fn root_lease(&self) -> &RootLease {
        &self.lease
    }
    pub(crate) async fn revalidate(&self) -> Result<(), WorkspaceError> {
        let directory = Arc::clone(&self.directory);
        let root_directory = Arc::clone(&self.root_directory);
        let expected = self.root_lease().root();
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
            .await
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
        // Refuse before canonicalize can interpret a relative path against the
        // daemon's ambient cwd (and before taking a blocking metadata permit).
        if !folder.is_absolute() {
            return Err(WorkspaceError::Directory(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "root must be absolute",
            )));
        }
        let source = folder.to_path_buf();
        let folder = self
            .root_metadata(move || std::fs::canonicalize(source))
            .await?;
        let opened = self.opened_admission(&folder).await?;
        let deadline = tokio::time::Instant::now()
            .checked_add(ADMISSION_TIMEOUT)
            .ok_or(RootRefusal::TimedOut)?;
        let lease = self.exclusion.claim(opened.ancestry, deadline).await?;
        Ok(StartGuard {
            directory: opened.directory,
            root_directory: opened.root_directory,
            workspaces: Arc::clone(self),
            lease,
        })
    }
}
