//! Which folder a git or file call works in: a project's root or a
//! worktree's, resolved and checked again on every call (a project folder
//! can be moved or replaced by a link between calls).

use vessel_proto::workspace::{ProjectState, RefName, WorkspaceRoot, WorktreeBase};
use vessel_proto::{ProjectId, WorktreeId};
use vessel_workspace::paths::Root;

use super::{WorkspaceError, Workspaces, policy};

/// A root resolved for one call.
#[derive(Debug)]
pub(crate) struct Resolved {
    /// The folder, resolved.
    pub(crate) root: Root,
    /// Whether it is a git repository.
    pub(crate) git: bool,
    /// Its project.
    pub(crate) project: ProjectId,
    /// The worktree, when the root is one.
    pub(crate) worktree: Option<WorktreeId>,
    /// The base `base` diffs and `git.status` compare with: a worktree's
    /// own (by name when it started from one), else the project's default
    /// branch.
    pub(crate) base: Option<RefName>,
}

impl Resolved {
    /// The base as a diff scope names one.
    pub(crate) fn base(&self) -> Option<WorktreeBase> {
        self.base.clone().map(|name| WorktreeBase::Ref { name })
    }
}

impl Workspaces {
    /// `root`, resolved: a ready project's folder or a worktree's.
    ///
    /// # Errors
    /// `not_found` for an unknown one; `conflict` for a project not ready;
    /// the root checks of `policy`.
    pub(crate) async fn resolve(&self, root: WorkspaceRoot) -> Result<Resolved, WorkspaceError> {
        let (path, git, project, worktree, base) = {
            let state = self.state();
            match root {
                WorkspaceRoot::Project { project } => {
                    let entry = state
                        .projects
                        .get(&project)
                        .ok_or(WorkspaceError::UnknownProject(project))?;
                    if !matches!(entry.info.state, ProjectState::Ready) {
                        return Err(WorkspaceError::Conflict(
                            "the project is not ready (cloning, failed, or its folder is gone)"
                                .to_owned(),
                        ));
                    }
                    (
                        entry.info.root.clone(),
                        entry.info.git,
                        project,
                        None,
                        entry.info.default_branch.clone(),
                    )
                }
                WorkspaceRoot::Worktree { worktree } => {
                    let entry = state
                        .worktrees
                        .get(&worktree)
                        .ok_or(WorkspaceError::UnknownWorktree(worktree))?;
                    let project = entry.info.project;
                    let default = state
                        .projects
                        .get(&project)
                        .and_then(|project| project.info.default_branch.clone());
                    let base = entry
                        .info
                        .base
                        .as_ref()
                        .and_then(|start| start.name.clone())
                        .or(default);
                    (entry.info.path.clone(), true, project, Some(worktree), base)
                }
            }
        };
        let opened = tokio::task::spawn_blocking(move || Root::open(&path))
            .await
            .map_err(|_panicked| WorkspaceError::Io("resolving the folder stopped".to_owned()))?
            .map_err(|_gone| {
                WorkspaceError::Conflict("the folder is gone from this machine".to_owned())
            })?;
        if worktree.is_none() {
            policy::check_root(&self.places, opened.path())?;
        }
        Ok(Resolved {
            root: opened,
            git,
            project,
            worktree,
            base,
        })
    }

    /// `root`, resolved, for a git call: refused for a folder without git.
    ///
    /// # Errors
    /// As [`resolve`](Self::resolve); `unsupported` without git.
    pub(crate) async fn resolve_git(
        &self,
        root: WorkspaceRoot,
    ) -> Result<Resolved, WorkspaceError> {
        let _git = self.git()?;
        let resolved = self.resolve(root).await?;
        if !resolved.git {
            return Err(WorkspaceError::NotGit);
        }
        Ok(resolved)
    }
}
