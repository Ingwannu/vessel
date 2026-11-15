//! What the node's pull requests need of a project's folder (protocol 11,
//! ADR 0029): the branch and commit checked out, the remote a branch
//! pushes to, where that remote last had the branch, the project's base
//! branch, and the push `pr.open` makes for a person. Thin: the git runs
//! are `vessel-workspace`'s, hardened as every other (ADR 0030).

use std::path::Path;

use vessel_proto::ProjectId;
use vessel_proto::workspace::{BranchName, CommitId, PushOutcome, RemoteName, SessionPlace};
use vessel_workspace::{Checkout, RemoteAddress};

use super::{WorkspaceError, Workspaces};

impl Workspaces {
    /// The commit and branch `folder` has checked out.
    pub(crate) async fn checkout(&self, folder: &Path) -> Result<Checkout, WorkspaceError> {
        Ok(self.git()?.checkout(folder).await?)
    }

    /// The remote `branch` pushes to in `folder`, and its address.
    pub(crate) async fn remote_address(
        &self,
        folder: &Path,
        branch: &BranchName,
    ) -> Result<Option<(RemoteName, RemoteAddress)>, WorkspaceError> {
        Ok(self.git()?.remote_address(folder, branch).await?)
    }

    /// Repository reads use origin when no branch/head exists. This is
    /// staged with the project listing; wiring removes the attribute.
    #[cfg(test)]
    pub(crate) async fn project_remote_address(
        &self,
        folder: &Path,
        branch: Option<&BranchName>,
    ) -> Result<Option<(RemoteName, RemoteAddress)>, WorkspaceError> {
        Ok(self.git()?.remote_for_checkout(folder, branch).await?)
    }

    /// Where `remote` was last seen to have `branch`.
    pub(crate) async fn tracking(
        &self,
        folder: &Path,
        remote: &RemoteName,
        branch: &BranchName,
    ) -> Result<Option<CommitId>, WorkspaceError> {
        Ok(self.git()?.tracking(folder, remote, branch).await?)
    }

    /// Push the branch `folder` has checked out to `remote`, setting its
    /// upstream; never forced. The repository's hooks run, as for
    /// `git.push` (a person's call only).
    pub(crate) async fn push_branch(
        &self,
        folder: &Path,
        remote: &RemoteName,
    ) -> Result<PushOutcome, WorkspaceError> {
        Ok(self.git()?.push(folder, Some(remote), true).await?)
    }

    /// The branch pull requests of `project` go into: its default branch,
    /// without its remote's name (`origin/main` is `main`).
    pub(crate) fn base_branch(&self, project: ProjectId) -> Option<BranchName> {
        let state = self.state();
        let default = state.projects.get(&project)?.info.default_branch.clone()?;
        let name = default.as_str();
        name.strip_prefix("origin/").unwrap_or(name).parse().ok()
    }
}

impl Workspaces {
    /// The project a session placed at `place` works in: the project's,
    /// or its worktree's.
    pub(crate) fn project_of(&self, place: &SessionPlace) -> Option<ProjectId> {
        match place {
            SessionPlace::Project { project, .. } => Some(*project),
            SessionPlace::Worktree { worktree } => self
                .state()
                .worktrees
                .get(worktree)
                .map(|entry| entry.info.project),
        }
    }
}
