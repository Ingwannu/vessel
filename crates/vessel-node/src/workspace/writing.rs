//! One ordinary file writer consumes admission in its retained node task.
//! Dropping an RPC receiver cannot drop the accepted I/O future or its guard.
use super::{StartGuard, WorkspaceError, Workspaces, roots::Resolved};
use std::{future::Future, sync::Arc};
use tokio::sync::oneshot;
impl Workspaces {
    pub(super) async fn run_file_write<T, Make, Work>(
        self: &Arc<Self>,
        root: vessel_proto::workspace::WorkspaceRoot,
        make: Make,
    ) -> Result<T, WorkspaceError>
    where
        T: Send + 'static,
        Make: FnOnce(Resolved) -> Work + Send + 'static,
        Work: Future<Output = Result<T, WorkspaceError>> + Send + 'static,
    {
        if self.stop.is_cancelled() {
            return Err(WorkspaceError::Stopping);
        }
        let resolved = self.resolve(root).await?;
        let guard = self.admit_start(resolved.root.path()).await?;
        let (reply, received) = oneshot::channel();
        let _retained = self.tasks.spawn(async move {
            let result = under_root(guard, resolved, make).await;
            let _gone = reply.send(result);
        });
        received.await.map_err(WorkspaceError::ReplyGone)?
    }
}

// Keep the typed guard owned until actual I/O and postconditions have returned.
async fn under_root<T, Make, Work>(
    guard: StartGuard,
    resolved: Resolved,
    make: Make,
) -> Result<T, WorkspaceError>
where
    Make: FnOnce(Resolved) -> Work,
    Work: Future<Output = Result<T, WorkspaceError>>,
{
    guard.revalidate().await?;
    let value = make(resolved).await?;
    guard.revalidate().await?;
    Ok(value)
}
