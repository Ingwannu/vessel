//! A project's snapshots: listing them and dropping one (ADR 0027). Making
//! one is part of a removal (`removal`), putting one back part of making a
//! worktree (`create`); git's side of both is `vessel-workspace`'s.

use vessel_proto::workspace::SnapshotList;
use vessel_proto::{ProjectId, WorktreeId};

use crate::workspace::{WorkspaceError, Workspaces};

impl Workspaces {
    /// A project's snapshots.
    ///
    /// # Errors
    /// `not_found`; `unsupported`.
    pub(crate) async fn list_snapshots(
        &self,
        project: ProjectId,
    ) -> Result<SnapshotList, WorkspaceError> {
        let root = self.project_root(project)?;
        Ok(self.git()?.snapshots(&root).await?)
    }

    /// Delete a snapshot's ref.
    ///
    /// # Errors
    /// `not_found` for a snapshot the project does not have.
    pub(crate) async fn drop_snapshot(
        &self,
        project: ProjectId,
        worktree: WorktreeId,
    ) -> Result<(), WorkspaceError> {
        let root = self.project_root(project)?;
        if self.git()?.drop_snapshot(&root, worktree).await? {
            Ok(())
        } else {
            Err(WorkspaceError::UnknownSnapshot(worktree))
        }
    }
}
