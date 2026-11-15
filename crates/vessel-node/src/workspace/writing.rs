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
    super::admission::OpenedAdmission,
    super::exclusion::{ADMISSION_TIMEOUT, RootLease, RootRefusal},
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
            let mut opened = self.opened_admission(resolved.root.path()).await?;
            let deadline = tokio::time::Instant::now()
                .checked_add(ADMISSION_TIMEOUT)
                .ok_or(RootRefusal::TimedOut)?;
            let lease = self
                .exclusion
                .claim_registered(opened.root, std::mem::take(&mut opened.worktrees), deadline)
                .await?;
            (opened, lease)
        };
        let (mut reply, received) = oneshot::channel();
        let cancellation = self.stop.child_token();
        // The tracker owns this task; cancelling its reply receiver never aborts
        // a child blocking syscall or releases an admitted writer early.
        #[cfg(target_os = "linux")]
        let workspaces = Arc::clone(self);
        let _retained = self.tasks.spawn(async move {
            #[cfg(target_os = "linux")]
            let operation = under_root(
                workspaces,
                resolved,
                make,
                directory,
                lease,
                cancellation.clone(),
            );
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
    workspaces: Arc<Workspaces>,
    resolved: Resolved,
    make: Make,
    directory: OpenedAdmission,
    lease: RootLease,
    cancellation: CancellationToken,
) -> Result<T, WorkspaceError>
where
    Make: FnOnce(Resolved, CancellationToken) -> Work,
    Work: Future<Output = Result<T, WorkspaceError>>,
{
    let result = async {
        let directory = revalidate_in(&workspaces, directory, lease.root()).await?;
        let value = execute(resolved, make, cancellation).await?;
        let _verified = revalidate_in(&workspaces, directory, lease.root()).await?;
        Ok(value)
    }
    .await;
    // The next operation may start as soon as the reply becomes visible.
    drop(lease);
    result
}
#[cfg(target_os = "linux")]
async fn revalidate_in(
    workspaces: &Workspaces,
    directory: OpenedAdmission,
    root: vessel_proto::checkpoint::PhysicalRoot,
) -> Result<OpenedAdmission, WorkspaceError> {
    let (directory, observed) = workspaces
        .root_metadata(move || {
            directory.root_directory.revalidate()?;
            directory.directory.revalidate()?;
            let observed = directory
                .directory
                .ancestry()?
                .nearest(&std::collections::BTreeSet::from([root]));
            Ok((directory, observed))
        })
        .await?;
    if observed != Some(root) {
        return Err(RootRefusal::RecoveryPending.into());
    }
    Ok(directory)
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
