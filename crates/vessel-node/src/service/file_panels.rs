//! File-panel routing has a bounded work budget and reuses existing
//! principal wait shares. Forwarded requests never hold a long poll.

use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use vessel_proto::workspace::directory_metadata::{InspectDirectory, InspectedDirectory};
use vessel_proto::workspace::panel_files::{
    DuplicateFileEntry, DuplicatedFileEntry, FileBytePage, ObserveFile, ObservedFile, ReadFileBytes,
};
use vessel_proto::workspace::suggestions::{EntrySuggestions, SuggestEntries};
use vessel_proto::{ErrorCode, WireError};

use super::projects::wire;
use super::{Connection, NodeService};
use crate::principal::Principal;
use crate::waits::Waiter;

/// Bound file hashing/copy/discovery independently of RPC frame concurrency.
const FILE_WORKERS: usize = 4;
/// Each finite observation owns at most two OS directory watches.
const FILE_OBSERVERS: usize = 64;
/// User waits end finitely; an uncancellable disk task retains its slot.
const FILE_OPERATION_DEADLINE: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub(crate) struct FilePanelSlots {
    work: Arc<Semaphore>,
    observers: Arc<Semaphore>,
}

impl Default for FilePanelSlots {
    fn default() -> Self {
        Self {
            work: Arc::new(Semaphore::new(FILE_WORKERS)),
            observers: Arc::new(Semaphore::new(FILE_OBSERVERS)),
        }
    }
}

impl FilePanelSlots {
    fn work(&self) -> Result<OwnedSemaphorePermit, WireError> {
        Arc::clone(&self.work).try_acquire_owned().map_err(|_busy| {
            WireError::new(ErrorCode::Unavailable, "workspace file operations are busy")
        })
    }
}

impl NodeService {
    async fn run_file_work<T: Send + 'static>(
        &self,
        work: impl Future<Output = Result<T, crate::workspace::WorkspaceError>> + Send + 'static,
    ) -> Result<T, WireError> {
        let permit = self.file_panels.work()?;
        let task = self.workspaces.spawn_file_work(owned_work(permit, work));
        tokio::time::timeout(FILE_OPERATION_DEADLINE, task)
            .await
            .map_err(|_expired| {
                WireError::new(ErrorCode::Unavailable, "workspace file operation timed out")
            })?
            .map_err(|_stopped| WireError::new(ErrorCode::Internal, "workspace file task stopped"))?
            .map_err(|error| wire(&error))
    }

    pub(crate) async fn read_file_bytes(
        &self,
        _: &Connection,
        request: ReadFileBytes,
    ) -> Result<FileBytePage, WireError> {
        let workspaces = Arc::clone(&self.workspaces);
        self.run_file_work(async move { workspaces.read_file_bytes(request).await })
            .await
    }

    pub(crate) async fn duplicate_file_entry(
        &self,
        _: &Connection,
        request: DuplicateFileEntry,
    ) -> Result<DuplicatedFileEntry, WireError> {
        let workspaces = Arc::clone(&self.workspaces);
        self.run_file_work(async move { workspaces.duplicate_file_entry(request).await })
            .await
    }

    pub(crate) async fn inspect_directory(
        &self,
        _: &Connection,
        request: InspectDirectory,
    ) -> Result<InspectedDirectory, WireError> {
        let workspaces = Arc::clone(&self.workspaces);
        self.run_file_work(async move { workspaces.inspect_directory(request).await })
            .await
    }

    pub(crate) async fn suggest_entries(
        &self,
        _: &Connection,
        request: SuggestEntries,
    ) -> Result<EntrySuggestions, WireError> {
        let workspaces = Arc::clone(&self.workspaces);
        self.run_file_work(async move { workspaces.suggest_entries(request).await })
            .await
    }

    pub(crate) async fn observe_file(
        &self,
        connection: &Connection,
        mut request: ObserveFile,
    ) -> Result<ObservedFile, WireError> {
        let permit = Arc::clone(&self.file_panels.observers)
            .try_acquire_owned()
            .map_err(|_busy| {
                WireError::new(ErrorCode::Unavailable, "workspace file observers are busy")
            })?;
        let mut wait = request
            .after
            .as_ref()
            .map(|_| u32::try_from(vessel_workspace::files::observe::OBSERVE_HEARTBEAT.as_secs()))
            .transpose()
            .map_err(|_overflow| {
                WireError::new(ErrorCode::Internal, "file observation duration is invalid")
            })?;
        if matches!(connection.principal(), Principal::Delegated { .. }) {
            wait = None;
        }
        let here = self.links.node();
        let shares = Waiter::of(connection.principal(), here).shares(here);
        let waiting_slot = self.wait_slot(connection, shares, &mut wait);
        if wait.is_none() {
            request.after = None;
        }
        let workspaces = Arc::clone(&self.workspaces);
        let task = self
            .workspaces
            .spawn_file_work(owned_work(permit, async move {
                let _wait = waiting_slot;
                workspaces.observe_file(request).await
            }));
        tokio::time::timeout(FILE_OPERATION_DEADLINE, task)
            .await
            .map_err(|_expired| {
                WireError::new(
                    ErrorCode::Unavailable,
                    "workspace file observation timed out",
                )
            })?
            .map_err(|_stopped| {
                WireError::new(ErrorCode::Internal, "workspace file observer stopped")
            })?
            .map_err(|error| wire(&error))
    }
}

// The permit lives inside the tracked task. Dropping or timing out its
// caller cannot admit unlimited still-running blocking disk operations.
async fn owned_work<T>(permit: OwnedSemaphorePermit, work: impl Future<Output = T>) -> T {
    let _permit = permit;
    work.await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_dropped_caller_cannot_release_a_still_running_file_task_slot() {
        let slots = FilePanelSlots::default();
        let tracker = tokio_util::task::TaskTracker::new();
        let (release, waiting) = tokio::sync::oneshot::channel::<()>();
        let task = tracker.spawn(owned_work(slots.work().unwrap(), async move {
            let _released = waiting.await;
        }));
        drop(task);
        assert_eq!(slots.work.available_permits(), FILE_WORKERS - 1);
        release.send(()).unwrap();
        let _closed = tracker.close();
        tokio::time::timeout(FILE_OPERATION_DEADLINE, tracker.wait())
            .await
            .unwrap();
        assert_eq!(slots.work.available_permits(), FILE_WORKERS);
    }
}
