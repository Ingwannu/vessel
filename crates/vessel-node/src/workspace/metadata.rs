//! A blocked root metadata read retains its bounded worker permit after RPC loss.
use super::{WorkspaceError, Workspaces};
use std::sync::Arc;
use tokio::sync::TryAcquireError;
/// Independent from capture jobs; ordinary starts cannot create unlimited I/O.
pub(super) const ROOT_METADATA_WORKERS: usize = 2;
impl Workspaces {
    pub(super) async fn root_metadata<T, Read>(&self, read: Read) -> Result<T, WorkspaceError>
    where
        T: Send + 'static,
        Read: FnOnce() -> std::io::Result<T> + Send + 'static,
    {
        if self.stop.is_cancelled() {
            return Err(WorkspaceError::Stopping);
        }
        let permit =
            Arc::clone(&self.root_reads)
                .try_acquire_owned()
                .map_err(|error| match error {
                    TryAcquireError::Closed => WorkspaceError::Stopping,
                    TryAcquireError::NoPermits => WorkspaceError::MetadataBusy,
                })?;
        self.tasks
            .spawn_blocking(move || {
                let _permit = permit;
                read()
            })
            .await
            .map_err(WorkspaceError::project_task)?
            .map_err(WorkspaceError::Directory)
    }
}
