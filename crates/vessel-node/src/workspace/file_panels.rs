//! Registered file-panel operations reuse workspace resolution, never an
//! absolute path supplied by the caller. Disk work stays in vessel-workspace.

use std::sync::Arc;

use vessel_proto::workspace::directory_metadata::{InspectDirectory, InspectedDirectory};
use vessel_proto::workspace::panel_files::{
    DuplicateFileEntry, DuplicatedFileEntry, FileBytePage, ObserveFile, ObservedFile, ReadFileBytes,
};
use vessel_proto::workspace::suggestions::{EntrySuggestions, SuggestEntries};
use vessel_workspace::files;

use super::{WorkspaceError, Workspaces};

impl Workspaces {
    /// File work outlives a disconnected caller while keeping its resource
    /// permit, and is drained by the existing workspace task tracker.
    pub(crate) fn spawn_file_work<T: Send + 'static>(
        &self,
        work: impl Future<Output = T> + Send + 'static,
    ) -> tokio::task::JoinHandle<T> {
        self.tasks.spawn(work)
    }

    pub(crate) async fn read_file_bytes(
        &self,
        request: ReadFileBytes,
    ) -> Result<FileBytePage, WorkspaceError> {
        let resolved = self.resolve(request.root).await?;
        Ok(files::binary::read_bytes(
            &resolved.root,
            &request.path,
            request.offset,
            request.expected.as_ref(),
        )
        .await?)
    }

    /// Capture before file-task admission; use the same retained physical
    /// lifetime and cache guard as the legacy file mutations.
    pub(crate) fn duplicate_file_entry(
        self: &Arc<Self>,
        request: DuplicateFileEntry,
    ) -> impl Future<Output = Result<DuplicatedFileEntry, WorkspaceError>> + Send + 'static {
        self.mutating_file(request.root, move |opened| async move {
            let path = files::duplicate::duplicate(&opened, &request.path).await?;
            Ok(DuplicatedFileEntry { path })
        })
    }

    pub(crate) async fn observe_file(
        &self,
        request: ObserveFile,
    ) -> Result<ObservedFile, WorkspaceError> {
        let resolved = self.resolve(request.root).await?;
        Ok(files::observe::observe(&resolved.root, &request.path, request.after.as_ref()).await?)
    }

    pub(crate) async fn suggest_entries(
        &self,
        request: SuggestEntries,
    ) -> Result<EntrySuggestions, WorkspaceError> {
        let resolved = self.resolve(request.root).await?;
        let git = if resolved.git {
            self.git.as_ref()
        } else {
            None
        };
        Ok(files::suggestions::suggest(git, &resolved.root, &request).await?)
    }

    pub(crate) async fn inspect_directory(
        &self,
        request: InspectDirectory,
    ) -> Result<InspectedDirectory, WorkspaceError> {
        let resolved = self.resolve(request.root).await?;
        let git = if resolved.git {
            self.git.as_ref()
        } else {
            None
        };
        Ok(files::inspect::inspect(git, &resolved.root, &request).await?)
    }
}
