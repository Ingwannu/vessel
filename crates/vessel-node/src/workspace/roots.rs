//! Which folder a git or file call works in: the recorded root is
//! checked on every call. The shared blocking open also retains an
//! admission guard until its actual completion, including cancellation.

use super::{WorkspaceError, Workspaces, policy};
use std::path::{Path, PathBuf};
use vessel_proto::workspace::{ProjectState, RefName, WorkspaceRoot, WorktreeBase};
use vessel_proto::{ProjectId, WorktreeId};
use vessel_workspace::paths::Root;

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

#[derive(Debug, Clone, Copy)]
enum Kind {
    Git,
    Plain,
}

// One registry snapshot crosses the blocking boundary; resolving it
// again afterwards could bind the opened folder to another record.
struct Record {
    path: PathBuf,
    kind: Kind,
    repository: Option<PathBuf>,
    project: ProjectId,
    worktree: Option<WorktreeId>,
    base: Option<RefName>,
}

impl Workspaces {
    /// `root`, resolved: a ready project's folder or a worktree's.
    ///
    /// # Errors
    /// `not_found` for an unknown one; `conflict` for a project not ready;
    /// the root checks of `policy`.
    pub(crate) async fn resolve(&self, root: WorkspaceRoot) -> Result<Resolved, WorkspaceError> {
        self.resolve_keeping(root, ()).await
    }

    fn resolve_record(&self, root: WorkspaceRoot) -> Result<Record, WorkspaceError> {
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
                Ok(Record {
                    path: entry.info.root.clone(),
                    kind: if entry.info.git {
                        Kind::Git
                    } else {
                        Kind::Plain
                    },
                    repository: None,
                    project,
                    worktree: None,
                    base: entry.info.default_branch.clone(),
                })
            }
            WorkspaceRoot::Worktree { worktree } => {
                let entry = state
                    .worktrees
                    .get(&worktree)
                    .ok_or(WorkspaceError::UnknownWorktree(worktree))?;
                let project = entry.info.project;
                let owner = state
                    .projects
                    .get(&project)
                    .ok_or(WorkspaceError::UnknownProject(project))?;
                let base = entry
                    .info
                    .base
                    .as_ref()
                    .and_then(|start| start.name.clone())
                    .or_else(|| owner.info.default_branch.clone());
                Ok(Record {
                    path: entry.info.path.clone(),
                    kind: Kind::Git,
                    repository: Some(owner.info.root.clone()),
                    project,
                    worktree: Some(worktree),
                    base,
                })
            }
        }
    }

    async fn resolve_keeping<K: Send + 'static>(
        &self,
        root: WorkspaceRoot,
        keep: K,
    ) -> Result<Resolved, WorkspaceError> {
        let record = self.resolve_record(root)?;
        let path = record.path;
        let repository = record.repository;
        // The result and guard travel together. Dropping this awaiter
        // cannot drop admission while open is queued or running.
        let (opened, keep) = tokio::task::spawn_blocking(move || {
            let opened = open(&path, repository.as_deref());
            (opened, keep)
        })
        .await
        .map_err(WorkspaceError::ResolveTask)?;
        let opened = opened?;
        if record.worktree.is_none() {
            policy::check_root(&self.places, opened.path())?;
        }
        let resolved = Resolved {
            root: opened,
            git: matches!(record.kind, Kind::Git),
            project: record.project,
            worktree: record.worktree,
            base: record.base,
        };
        drop(keep);
        Ok(resolved)
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

    /// Staged project reads retain admission through the same root open;
    /// reviewed transport wiring removes this staging attribute.
    #[cfg(test)]
    pub(crate) async fn resolve_git_guarded<K: Send + 'static>(
        &self,
        root: WorkspaceRoot,
        keep: K,
    ) -> Result<Resolved, WorkspaceError> {
        let _git = self.git()?;
        let resolved = self.resolve_keeping(root, keep).await?;
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
