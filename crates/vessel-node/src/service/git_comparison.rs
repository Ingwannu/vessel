//! Pending ADR0087 methods route to the same workspace authority as legacy git.
use vessel_proto::WireError;
use vessel_proto::workspace::{CompareGit, CompareGitFile, DiffSummary, FileDiff, GitCompared};

use super::projects::wire;
use super::{Connection, NodeService};

impl NodeService {
    pub(crate) async fn git_compare(
        &self,
        _: &Connection,
        request: CompareGit,
    ) -> Result<GitCompared<DiffSummary>, WireError> {
        self.workspaces
            .git_compare(request)
            .await
            .map_err(|error| wire(&error))
    }

    pub(crate) async fn git_compare_file(
        &self,
        _: &Connection,
        request: CompareGitFile,
    ) -> Result<GitCompared<FileDiff>, WireError> {
        self.workspaces
            .git_compare_file(request)
            .await
            .map_err(|error| wire(&error))
    }
    pub(crate) async fn git_history(
        &self,
        _: &Connection,
        request: vessel_proto::workspace::ReadGitHistory,
    ) -> Result<vessel_proto::workspace::GitHistoryRead, WireError> {
        self.workspaces
            .git_history(request)
            .await
            .map_err(|error| wire(&error))
    }
}
