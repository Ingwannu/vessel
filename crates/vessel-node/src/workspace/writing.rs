//! Retained file operations keep root admission after a caller loses its response.
use super::{WorkspaceError, Workspaces, roots::Resolved};
use std::{future::Future, sync::Arc};
use tokio::sync::oneshot;
use vessel_proto::workspace::WorkspaceRoot;
#[cfg(target_os = "linux")]
use {
    super::exclusion::{ADMISSION_TIMEOUT, RootLease, RootRefusal},
    vessel_workspace::physical::Directory,
};

impl Workspaces {
    pub(super) async fn run_file_write<T, Make, Work>(
        self: &Arc<Self>,
        root: WorkspaceRoot,
        make: Make,
    ) -> Result<T, WorkspaceError>
    where
        T: Send + 'static,
        Make: FnOnce(Resolved) -> Work + Send + 'static,
        Work: Future<Output = Result<T, WorkspaceError>> + Send + 'static,
    {
        let resolved = self.resolve(root).await?;
        #[cfg(target_os = "linux")]
        let (directory, lease) = {
            let path = resolved.root.path().to_path_buf();
            let directory = tokio::task::spawn_blocking(move || Directory::open(&path))
                .await?
                .map_err(WorkspaceError::Directory)?;
            let deadline = tokio::time::Instant::now()
                .checked_add(ADMISSION_TIMEOUT)
                .ok_or(RootRefusal::TimedOut)?;
            let lease = self.exclusion.claim(directory.identity(), deadline).await?;
            (directory, lease)
        };
        let (reply, received) = oneshot::channel();
        // The tracker owns this task; cancelling its reply receiver never aborts
        // a child blocking syscall or releases an admitted writer early.
        let _retained = self.tasks.spawn(async move {
            #[cfg(target_os = "linux")]
            let result = under_root(resolved, make, directory, lease).await;
            #[cfg(not(target_os = "linux"))]
            let result = make(resolved).await;
            let _gone = reply.send(result);
        });
        received.await.map_err(WorkspaceError::ReplyGone)?
    }
}
#[cfg(target_os = "linux")]
async fn under_root<T, Make, Work>(
    resolved: Resolved,
    make: Make,
    directory: Directory,
    lease: RootLease,
) -> Result<T, WorkspaceError>
where
    Make: FnOnce(Resolved) -> Work,
    Work: Future<Output = Result<T, WorkspaceError>>,
{
    let result = async {
        if directory.identity() != lease.root() {
            return Err(RootRefusal::RecoveryPending.into());
        }
        let directory = revalidate(directory).await?;
        let value = make(resolved).await?;
        let _verified = revalidate(directory).await?;
        Ok(value)
    }
    .await;
    // The next operation may start as soon as the reply becomes visible.
    drop(lease);
    result
}
#[cfg(target_os = "linux")]
async fn revalidate(directory: Directory) -> Result<Directory, WorkspaceError> {
    tokio::task::spawn_blocking(move || {
        directory.revalidate()?;
        Ok(directory)
    })
    .await?
    .map_err(WorkspaceError::Directory)
}
