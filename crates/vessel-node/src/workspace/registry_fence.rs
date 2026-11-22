//! Legacy cleanup never bypasses retained independent workspace/backing ownership.
//! This is metadata serialization, not exit, generation or physical-root proof.
use super::{WorkspaceError, Workspaces};
use vessel_proto::{ProjectId, WorktreeId};
use vessel_store::WorkspaceArchiveClaim;

impl Workspaces {
    /// Must run under the same retained project metadata admission as cleanup.
    pub(super) async fn require_no_registry_reference(
        &self,
        project: ProjectId,
    ) -> Result<(), WorkspaceError> {
        if self
            .store
            .registry_project_reference(project)
            .await?
            .is_some()
        {
            return Err(WorkspaceError::RegistryProjectOwned(project));
        }
        Ok(())
    }

    /// The retained project admission serializes this check with reservation
    /// and backing publication. A claim permits removal only for its backing.
    pub(super) async fn check_backing_removal(
        &self,
        worktree: WorktreeId,
        claim: Option<WorkspaceArchiveClaim>,
    ) -> Result<(), WorkspaceError> {
        let allowed = match claim {
            Some(claim) => self.store.registry_archive_owns_worktree(claim, worktree).await?,
            None => self.store.registry_worktree_reference(worktree).await?.is_none(),
        };
        if !allowed {
            return Err(WorkspaceError::RegistryWorktreeOwned(worktree));
        }
        Ok(())
    }
}
