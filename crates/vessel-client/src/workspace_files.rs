//! File-panel calls use the same generated unary contract as every other
//! request. A deadline bounds the local await, not the remote effect lifetime.
//! A timed-out mutation needs original terminal evidence before retrying;
//! refreshing alone cannot prove the old operation ended.

use std::time::Duration;
use vessel_proto::workspace::directory_metadata::{InspectDirectory, InspectedDirectory};
use vessel_proto::workspace::panel_files::{
    DuplicateFileEntry, DuplicatedFileEntry, FileBytePage, ObserveFile, ObservedFile, ReadFileBytes,
};
use vessel_proto::workspace::suggestions::{EntrySuggestions, SuggestEntries};

use crate::{CallError, NodeClient, Unary};

/// Longer than the node's operation budget, leaving time for a final reply.
const FILE_CALL_DEADLINE: Duration = Duration::from_secs(70);

/// Call failures stay typed; a timeout is not proof that a mutation failed.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceFileCallError {
    /// Authentication, admission, transport or remote filesystem refusal.
    #[error(transparent)]
    Call(#[from] CallError),
    /// The remote operation may still finish; a new read does not prove it ended.
    #[error("the file operation timed out; its remote completion is unknown")]
    Deadline,
}

impl NodeClient {
    /// Read an exact bounded byte page, guarded by its original revision.
    ///
    /// # Errors
    /// Older/unassigned protocols and filesystem failures remain typed.
    pub async fn read_workspace_byte_page(
        &self,
        request: ReadFileBytes,
    ) -> Result<FileBytePage, WorkspaceFileCallError> {
        file_call(self, request).await
    }

    /// List directory metadata for name, modification-time and size sorting.
    ///
    /// # Errors
    /// As [`Self::read_workspace_byte_page`].
    pub async fn inspect_workspace_directory(
        &self,
        request: InspectDirectory,
    ) -> Result<InspectedDirectory, WorkspaceFileCallError> {
        file_call(self, request).await
    }

    /// Create the first exclusively available sibling copy without overwrite.
    ///
    /// # Errors
    /// Links, special files, root targets and oversized copies are refused.
    pub async fn duplicate_workspace_entry(
        &self,
        request: DuplicateFileEntry,
    ) -> Result<DuplicatedFileEntry, WorkspaceFileCallError> {
        file_call(self, request).await
    }

    /// Observe saved state with the node's finite parent-directory heartbeat.
    ///
    /// # Errors
    /// A watch failure or budget refusal is never mistaken for a quiet file.
    pub async fn observe_workspace_file(
        &self,
        request: ObserveFile,
    ) -> Result<ObservedFile, WorkspaceFileCallError> {
        file_call(self, request).await
    }

    /// Discover files/directories under one already authorized workspace root.
    ///
    /// # Errors
    /// Input decode, root resolution and discovery errors remain visible.
    pub async fn suggest_workspace_entries(
        &self,
        request: SuggestEntries,
    ) -> Result<EntrySuggestions, WorkspaceFileCallError> {
        file_call(self, request).await
    }
}

async fn file_call<R: Unary>(
    client: &NodeClient,
    request: R,
) -> Result<R::Response, WorkspaceFileCallError> {
    tokio::time::timeout(FILE_CALL_DEADLINE, client.call(request))
        .await
        .map_err(|_expired| WorkspaceFileCallError::Deadline)?
        .map_err(WorkspaceFileCallError::Call)
}
