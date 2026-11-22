//! Legacy cleanup never bypasses retained independent workspace/backing ownership.
//! This is metadata serialization, not exit, generation or physical-root proof.
use super::{WorkspaceError, Workspaces};
use vessel_proto::{ProjectId, WorkspaceId, WorktreeId};
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
        requested: bool,
    ) -> Result<Option<WorkspaceId>, WorkspaceError> {
        if let Some(claim) = claim {
            return if self
                .store
                .registry_archive_owns_worktree(claim, worktree)
                .await?
            {
                Ok(None)
            } else {
                Err(WorkspaceError::RegistryWorktreeOwned(worktree))
            };
        }
        if self
            .store
            .registry_worktree_reference(worktree)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        if requested && let Some(debt) = self.store.archived_worktree_debt(worktree).await? {
            return Ok(Some(debt));
        }
        Err(WorkspaceError::RegistryWorktreeOwned(worktree))
    }

    /// Refuse before the optional terminal-stop request. Removal rechecks
    /// under its retained project admission before touching the backing.
    pub(crate) async fn check_requested_worktree_removal(
        &self,
        worktree: WorktreeId,
    ) -> Result<(), WorkspaceError> {
        let _debt = self.check_backing_removal(worktree, None, true).await?;
        Ok(())
    }
}
