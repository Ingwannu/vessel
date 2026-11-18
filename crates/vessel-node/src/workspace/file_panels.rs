//! Registered file-panel operations reuse workspace resolution, never an
//! absolute path supplied by the caller. Disk work stays in vessel-workspace.

use vessel_proto::workspace::directory_metadata::{InspectDirectory, InspectedDirectory};
use vessel_proto::workspace::host_folders::{
    FolderSuggestions, GetProjectIcon, ProjectIcon, SuggestFolders,
};
use vessel_proto::workspace::panel_files::{
    DuplicateFileEntry, DuplicatedFileEntry, FileBytePage, ObserveFile, ObservedFile, ReadFileBytes,
};
use vessel_proto::workspace::suggestions::{EntrySuggestions, SuggestEntries};
use vessel_workspace::files;
use vessel_workspace::paths::Root;

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

    pub(crate) async fn duplicate_file_entry(
        &self,
        request: DuplicateFileEntry,
    ) -> Result<DuplicatedFileEntry, WorkspaceError> {
        let resolved = self.resolve(request.root).await?;
        let path = files::duplicate::duplicate(&resolved.root, &request.path).await?;
        Ok(DuplicatedFileEntry { path })
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

    /// `folder.suggest` (ADR 0112): under the node user's home, resolved
    /// as the node started.
    pub(crate) async fn suggest_folders(
        &self,
        request: SuggestFolders,
    ) -> Result<FolderSuggestions, WorkspaceError> {
        Ok(files::host_folders::suggest_folders(&self.places.home, &request).await?)
    }

    /// `project.icon` (ADR 0112): inside the registered project's root; a
    /// root that cannot be opened (moved, removed) has no icon.
    pub(crate) async fn project_icon(
        &self,
        request: GetProjectIcon,
    ) -> Result<ProjectIcon, WorkspaceError> {
        let root = self.project_root(request.project)?;
        let Ok(root) = tokio::task::spawn_blocking(move || Root::open(&root))
            .await
            .map_err(|_stopped| WorkspaceError::from(files::FileError::Task))?
        else {
            return Ok(ProjectIcon { icon: None });
        };
        Ok(files::project_icon::project_icon(&root).await?)
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
