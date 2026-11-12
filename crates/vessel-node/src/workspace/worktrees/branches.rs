//! Worktrees following their branches: the record keeps up with a branch
//! renamed (`git.rename_branch`) or switched (`git.switch`) by a call
//! here. It runs no git; the calls that change a branch are the
//! workspace's `git`.

use vessel_proto::workspace::{BranchName, WorktreeInfo};
use vessel_proto::{ProjectId, WorktreeId};

use crate::workspace::Workspaces;

impl Workspaces {
    /// The project's worktrees follow `git.rename_branch`: a branch is the
    /// repository's, so every worktree that has it checked out, or created
    /// it, records the new name.
    pub(crate) async fn branch_renamed(
        &self,
        project: ProjectId,
        from: &BranchName,
        to: &BranchName,
    ) {
        let changed: Vec<WorktreeInfo> = {
            let mut state = self.state();
            state
                .worktrees
                .values_mut()
                .filter(|entry| entry.info.project == project)
                .filter_map(|entry| {
                    let mut changed = false;
                    if entry.info.branch.as_ref() == Some(from) {
                        entry.info.branch = Some(to.clone());
                        changed = true;
                    }
                    if entry.info.created_branch.as_ref() == Some(from) {
                        entry.info.created_branch = Some(to.clone());
                        changed = true;
                    }
                    changed.then(|| entry.info.clone())
                })
                .collect()
        };
        for info in changed {
            if let Err(error) = self.save_worktree(info).await {
                tracing::warn!(%error, "could not record a renamed branch");
            }
        }
    }

    /// A worktree's branch after `git.switch` in it.
    pub(crate) async fn branch_switched(&self, worktree: WorktreeId, now: Option<BranchName>) {
        let changed = {
            let mut state = self.state();
            state.worktrees.get_mut(&worktree).and_then(|entry| {
                (entry.info.branch != now).then(|| {
                    entry.info.branch = now;
                    entry.info.clone()
                })
            })
        };
        if let Some(info) = changed
            && let Err(error) = self.save_worktree(info).await
        {
            tracing::warn!(%error, "could not record a switched branch");
        }
    }
}
