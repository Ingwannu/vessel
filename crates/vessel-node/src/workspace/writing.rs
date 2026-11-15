//! Retained file operations keep root admission after a caller loses its response.
use super::{WorkspaceError, Workspaces, roots::Resolved};
use std::{future::Future, sync::Arc};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use vessel_proto::workspace::WorkspaceRoot;

#[derive(Debug, Clone, Copy)]
pub(super) enum RequiredRoot {
    Directory,
    Repository,
}
#[cfg(target_os = "linux")]
use {
    super::exclusion::{ADMISSION_TIMEOUT, RootLease, RootRefusal},
    vessel_workspace::physical::Directory,
};

impl Workspaces {
    pub(super) async fn run_write<T, Make, Work>(
        self: &Arc<Self>,
        root: WorkspaceRoot,
        required: RequiredRoot,
        make: Make,
    ) -> Result<T, WorkspaceError>
    where
        T: Send + 'static,
        Make: FnOnce(Resolved, CancellationToken) -> Work + Send + 'static,
        Work: Future<Output = Result<T, WorkspaceError>> + Send + 'static,
    {
        if self.stop.is_cancelled() {
            return Err(WorkspaceError::Stopping);
        }
        let resolved = self.resolve(root).await?;
        match required {
            RequiredRoot::Directory => {}
            RequiredRoot::Repository => {
                if !resolved.git {
                    return Err(WorkspaceError::NotGit);
                }
                let _git = self.git()?;
            }
        }
        #[cfg(target_os = "linux")]
        let (directory, lease) = {
            let path = resolved.root.path().to_path_buf();
            let directory = self.root_metadata(move || Directory::open(&path)).await?;
            let deadline = tokio::time::Instant::now()
                .checked_add(ADMISSION_TIMEOUT)
                .ok_or(RootRefusal::TimedOut)?;
            let lease = self.exclusion.claim(directory.identity(), deadline).await?;
            (directory, lease)
        };
        let (mut reply, received) = oneshot::channel();
        let cancellation = self.stop.child_token();
        // The tracker owns this task; cancelling its reply receiver never aborts
        // a child blocking syscall or releases an admitted writer early.
        let _retained = self.tasks.spawn(async move {
            #[cfg(target_os = "linux")]
            let operation = under_root(resolved, make, directory, lease, cancellation.clone());
            #[cfg(not(target_os = "linux"))]
            let operation = execute(resolved, make, cancellation.clone());
            tokio::pin!(operation);
            let result = tokio::select! {
                result=&mut operation=>result,
                ()=reply.closed()=>{cancellation.cancel();operation.await},
            };
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
    cancellation: CancellationToken,
) -> Result<T, WorkspaceError>
where
    Make: FnOnce(Resolved, CancellationToken) -> Work,
    Work: Future<Output = Result<T, WorkspaceError>>,
{
    let result = async {
        if directory.identity() != lease.root() {
            return Err(RootRefusal::RecoveryPending.into());
        }
        let directory = revalidate(directory).await?;
        let value = execute(resolved, make, cancellation).await?;
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

async fn execute<T, Make, Work>(
    resolved: Resolved,
    make: Make,
    cancellation: CancellationToken,
) -> Result<T, WorkspaceError>
where
    Make: FnOnce(Resolved, CancellationToken) -> Work,
    Work: Future<Output = Result<T, WorkspaceError>>,
{
    if cancellation.is_cancelled() {
        return Err(WorkspaceError::WriteCancelled);
    }
    make(resolved, cancellation).await
}
