//! Which folder a git or file call works in: a project's root or a
//! worktree's, resolved and checked again on every call (a project folder
//! can be moved or replaced by a link between calls).

use std::path::{Path, PathBuf};

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
    /// The worktree’s recorded ref before the project-default fallback.
    /// Comparison admission needs this fact from the same authority snapshot.
    pub(crate) pinned_base: Option<RefName>,
    /// Project default from that same registry snapshot, before any fallback.
    pub(crate) project_default: Option<RefName>,
}

impl Resolved {
    /// The base as a diff scope names one.
    pub(crate) fn base(&self) -> Option<WorktreeBase> {
        self.base.clone().map(|name| WorktreeBase::Ref { name })
    }
}

/// Registry facts only, captured under one short lock with no disk access.
/// This is a candidate identity, never authorization to skip resolve/open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub(super) path: PathBuf,
    pub(super) git: bool,
    repository: Option<PathBuf>,
    pub(super) project: ProjectId,
    worktree: Option<WorktreeId>,
    base: Option<RefName>,
    pinned_base: Option<RefName>,
    project_default: Option<RefName>,
}

fn project_snapshot(state: &super::State, project: ProjectId) -> Result<Snapshot, WorkspaceError> {
    let entry = state
        .projects
        .get(&project)
        .ok_or(WorkspaceError::UnknownProject(project))?;
    if !matches!(entry.info.state, ProjectState::Ready) {
        return Err(WorkspaceError::Conflict(
            "the project is not ready (cloning, failed, or its folder is gone)".to_owned(),
        ));
    }
    Ok(Snapshot {
        path: entry.info.root.clone(),
        git: entry.info.git,
        repository: None,
        project,
        worktree: None,
        base: entry.info.default_branch.clone(),
        pinned_base: None,
        project_default: entry.info.default_branch.clone(),
    })
}
fn worktree_snapshot(
    state: &super::State,
    worktree: WorktreeId,
) -> Result<Snapshot, WorkspaceError> {
    let entry = state
        .worktrees
        .get(&worktree)
        .ok_or(WorkspaceError::UnknownWorktree(worktree))?;
    let project = entry.info.project;
    let owner = state
        .projects
        .get(&project)
        .ok_or(WorkspaceError::UnknownProject(project))?;
    let pinned_base = entry
        .info
        .base
        .as_ref()
        .and_then(|start| start.name.clone());
    let base = pinned_base
        .clone()
        .or_else(|| owner.info.default_branch.clone());
    Ok(Snapshot {
        path: entry.info.path.clone(),
        git: true,
        repository: Some(owner.info.root.clone()),
        project,
        worktree: Some(worktree),
        base,
        pinned_base,
        project_default: owner.info.default_branch.clone(),
    })
}

impl Workspaces {
    /// Candidate facts for admission only; callers must resolve again before I/O.
    pub(super) fn root_snapshot(&self, root: WorkspaceRoot) -> Result<Snapshot, WorkspaceError> {
        let state = self.state();
        match root {
            WorkspaceRoot::Project { project } => project_snapshot(&state, project),
            WorkspaceRoot::Worktree { worktree } => worktree_snapshot(&state, worktree),
        }
    }

    /// `root`, resolved: a ready project's folder or a worktree's.
    ///
    /// # Errors
    /// `not_found` for an unknown one; `conflict` for a project not ready;
    /// the root checks of `policy`.
    pub(crate) async fn resolve(&self, root: WorkspaceRoot) -> Result<Resolved, WorkspaceError> {
        self.resolve_snapshot(self.root_snapshot(root)?).await
    }

    /// A queued operation cannot follow registry changes made while it waited.
    /// Capture once, compare every authority fact, then open that same snapshot.
    pub(super) async fn resolve_unchanged(
        &self,
        root: WorkspaceRoot,
        before: &Snapshot,
    ) -> Result<Resolved, WorkspaceError> {
        let current = self.root_snapshot(root)?;
        if &current != before {
            return Err(WorkspaceError::GitTargetChanged);
        }
        let resolved = self.resolve_snapshot(current).await?;
        if !resolved.git {
            return Err(WorkspaceError::NotGit);
        }
        Ok(resolved)
    }

    /// The open and its base/default facts consume one authority snapshot.
    pub(super) async fn resolve_snapshot(
        &self,
        snapshot: Snapshot,
    ) -> Result<Resolved, WorkspaceError> {
        let Snapshot {
            path,
            git,
            repository,
            project,
            worktree,
            base,
            pinned_base,
            project_default,
        } = snapshot;
        let opened = tokio::task::spawn_blocking(move || open(&path, repository.as_deref()))
            .await
            .map_err(|_panicked| WorkspaceError::Io("resolving the folder stopped".to_owned()))??;
        if worktree.is_none() {
            policy::check_root(&self.places, opened.path())?;
        }
        Ok(Resolved {
            root: opened,
            git,
            project,
            worktree,
            base,
            pinned_base,
            project_default,
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

/// Open the recorded folder `path` for a call: still a folder, still where
/// it was recorded (not replaced by a link to another, such as the node's
/// data folder), and, for a worktree of the repository at `repository`,
/// still that repository's (`gitfile::registered`), so git never reads a
/// configuration someone else put in its place. Blocking.
fn open(path: &Path, repository: Option<&Path>) -> Result<Root, WorkspaceError> {
    let opened = Root::open(path).map_err(|_gone| {
        WorkspaceError::Conflict("the folder is gone from this machine".to_owned())
    })?;
    if opened.path() != path {
        return Err(WorkspaceError::Conflict(
            "the folder was replaced since it was recorded (a link to another?); it is not used"
                .to_owned(),
        ));
    }
    if let Some(repository) = repository
        && !vessel_workspace::git::gitfile::registered(path, repository)
    {
        return Err(WorkspaceError::Conflict(
            "this worktree's `.git` no longer names its repository's record of it; git is not \
             run in it"
                .to_owned(),
        ));
    }
    Ok(opened)
}
