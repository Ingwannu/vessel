//! The git calls in a project's or a worktree's folder (protocol 10,
//! ADR 0023): the root resolved and checked on every call, then
//! `vessel-workspace`'s run.

use vessel_proto::workspace::{
    BranchList, BranchName, CommitMessage, CommitOutcome, CommitPage, CommitStage, DiffScope,
    DiffSummary, FetchOutcome, FileDiff, FileRead, GitStatus, PathSet, PullOutcome, PushOutcome,
    RelPath, RemoteName, RenameOutcome, StashAction, StashList, StashOutcome, SwitchOutcome,
    WorkspaceRoot, WorktreeBase,
};
use vessel_workspace::GitError;
use vessel_workspace::files::{MAX_BLOB, blob_page};

use super::roots::Resolved;
use super::{WorkspaceError, Workspaces};

/// How a write call changes the index.
#[derive(Debug, Clone, Copy)]
pub(crate) enum IndexChange {
    Stage,
    Unstage,
    Discard { untracked: bool },
}

impl Workspaces {
    /// `git.status`, with the distance from the root's base.
    ///
    /// # Errors
    /// As [`resolve_git`](Self::resolve_git)'s and git's.
    pub(crate) async fn git_status(
        &self,
        root: WorkspaceRoot,
    ) -> Result<GitStatus, WorkspaceError> {
        let resolved = self.resolve_git(root).await?;
        self.status_of(&resolved).await
    }

    async fn status_of(&self, resolved: &Resolved) -> Result<GitStatus, WorkspaceError> {
        let git = self.git()?;
        let dir = resolved.root.path();
        let mut status = git.status(dir).await?;
        if let Some(base) = &resolved.base {
            status.base = git.distance(dir, base).await?;
        }
        Ok(status)
    }

    /// `git.changes`.
    ///
    /// # Errors
    /// `invalid_request` for a scope whose base does not resolve.
    pub(crate) async fn git_changes(
        &self,
        root: WorkspaceRoot,
        scope: &DiffScope,
        ignore_whitespace: bool,
    ) -> Result<DiffSummary, WorkspaceError> {
        let resolved = self.resolve_git(root).await?;
        let git = self.git()?;
        let dir = resolved.root.path();
        let scope = scoped(
            git.resolve_scope(dir, scope, resolved.base().as_ref())
                .await,
        )?;
        Ok(git.changes(dir, &scope, ignore_whitespace).await?)
    }

    /// `git.diff`: one file's hunks, from `from_hunk`.
    ///
    /// # Errors
    /// As [`git_changes`](Self::git_changes).
    pub(crate) async fn git_diff(
        &self,
        root: WorkspaceRoot,
        scope: &DiffScope,
        path: &RelPath,
        ignore_whitespace: bool,
        from_hunk: u32,
    ) -> Result<FileDiff, WorkspaceError> {
        let resolved = self.resolve_git(root).await?;
        let git = self.git()?;
        let dir = resolved.root.path();
        let scope = scoped(
            git.resolve_scope(dir, scope, resolved.base().as_ref())
                .await,
        )?;
        Ok(git
            .file_diff(dir, &scope, path, ignore_whitespace, from_hunk)
            .await?)
    }

    /// `git.show`: a page of a file at a commit or ref.
    ///
    /// # Errors
    /// `invalid_request` for `at` that names no commit or a file too large;
    /// `not_found` for a path the commit does not have.
    pub(crate) async fn git_show(
        &self,
        root: WorkspaceRoot,
        at: &WorktreeBase,
        path: &RelPath,
        offset: u64,
        limit: Option<u32>,
    ) -> Result<FileRead, WorkspaceError> {
        if path.is_root() {
            return Err(WorkspaceError::NoSuchPath(path.clone()));
        }
        let resolved = self.resolve_git(root).await?;
        let git = self.git()?;
        let dir = resolved.root.path();
        let commit = git
            .resolve(dir, at)
            .await?
            .ok_or_else(|| WorkspaceError::Invalid("`at` does not name a commit".to_owned()))?;
        match git.read_blob(dir, &commit, path.as_str(), MAX_BLOB).await {
            Ok(Some(bytes)) => Ok(blob_page(&bytes, offset, limit)),
            Ok(None) => Err(WorkspaceError::NoSuchPath(path.clone())),
            Err(GitError::OutputLimit) => Err(WorkspaceError::Invalid(format!(
                "`{path}` is larger than {MAX_BLOB} bytes at that commit"
            ))),
            Err(error) => Err(error.into()),
        }
    }

    /// `git.branches`.
    ///
    /// # Errors
    /// As git's.
    pub(crate) async fn git_branches(
        &self,
        root: WorkspaceRoot,
    ) -> Result<BranchList, WorkspaceError> {
        let resolved = self.resolve_git(root).await?;
        Ok(self.git()?.branches(resolved.root.path()).await?)
    }

    /// `git.log`.
    ///
    /// # Errors
    /// `invalid_request` for a `from` that does not resolve.
    pub(crate) async fn git_log(
        &self,
        root: WorkspaceRoot,
        from: Option<&WorktreeBase>,
        skip: u32,
        limit: Option<u32>,
    ) -> Result<CommitPage, WorkspaceError> {
        let resolved = self.resolve_git(root).await?;
        let git = self.git()?;
        let dir = resolved.root.path();
        if let Some(from) = from
            && git.resolve(dir, from).await?.is_none()
        {
            return Err(WorkspaceError::Invalid(
                "`from` does not name a commit".to_owned(),
            ));
        }
        Ok(git.log(dir, from, skip, limit).await?)
    }

    /// `git.stashes`.
    ///
    /// # Errors
    /// As git's.
    pub(crate) async fn git_stashes(
        &self,
        root: WorkspaceRoot,
    ) -> Result<StashList, WorkspaceError> {
        let resolved = self.resolve_git(root).await?;
        Ok(self.git()?.stashes(resolved.root.path()).await?)
    }

    /// `git.stage`, `git.unstage`, `git.discard`: the status after.
    ///
    /// # Errors
    /// `unavailable` when another git holds the index's lock.
    pub(crate) async fn git_index(
        &self,
        root: WorkspaceRoot,
        paths: &PathSet,
        change: IndexChange,
    ) -> Result<GitStatus, WorkspaceError> {
        let operation = self.locked_git(root).await?;
        let resolved = &operation.target;
        let _mutation =
            self.comparisons
                .mutating(root, resolved.project, super::comparison::Change::Working);
        let git = self.git()?;
        let dir = resolved.root.path();
        match change {
            IndexChange::Stage => git.stage(dir, paths).await?,
            IndexChange::Unstage => git.unstage(dir, paths).await?,
            IndexChange::Discard { untracked } => git.discard(dir, paths, untracked).await?,
        }
        self.status_of(resolved).await
    }

    /// `git.commit`.
    ///
    /// # Errors
    /// As git's.
    pub(crate) async fn git_commit(
        &self,
        root: WorkspaceRoot,
        message: &CommitMessage,
        stage: CommitStage,
    ) -> Result<CommitOutcome, WorkspaceError> {
        let operation = self.locked_git(root).await?;
        let resolved = &operation.target;
        let _mutation = self.comparisons.mutating(
            root,
            resolved.project,
            super::comparison::Change::Repository,
        );
        Ok(self
            .git()?
            .commit(resolved.root.path(), message, stage)
            .await?)
    }

    /// `git.fetch`.
    ///
    /// # Errors
    /// As git's.
    pub(crate) async fn git_fetch(
        &self,
        root: WorkspaceRoot,
        remote: Option<&RemoteName>,
    ) -> Result<FetchOutcome, WorkspaceError> {
        let operation = self.locked_git(root).await?;
        let resolved = &operation.target;
        let _mutation =
            self.comparisons
                .mutating(root, resolved.project, super::comparison::Change::History);
        Ok(self.git()?.fetch(resolved.root.path(), remote).await?)
    }

    /// `git.push`.
    ///
    /// # Errors
    /// As git's.
    pub(crate) async fn git_push(
        &self,
        root: WorkspaceRoot,
        remote: Option<&RemoteName>,
        set_upstream: bool,
    ) -> Result<PushOutcome, WorkspaceError> {
        let operation = self.locked_git(root).await?;
        let resolved = &operation.target;
        let _mutation =
            self.comparisons
                .mutating(root, resolved.project, super::comparison::Change::History);
        Ok(self
            .git()?
            .push(resolved.root.path(), remote, set_upstream)
            .await?)
    }

    /// `git.pull`.
    ///
    /// # Errors
    /// As git's.
    pub(crate) async fn git_pull(
        &self,
        root: WorkspaceRoot,
    ) -> Result<PullOutcome, WorkspaceError> {
        let operation = self.locked_git(root).await?;
        let resolved = &operation.target;
        let _mutation = self.comparisons.mutating(
            root,
            resolved.project,
            super::comparison::Change::Repository,
        );
        Ok(self.git()?.pull(resolved.root.path()).await?)
    }

    /// `git.switch`; a worktree's recorded branch follows.
    ///
    /// # Errors
    /// As git's.
    pub(crate) async fn git_switch(
        &self,
        root: WorkspaceRoot,
        branch: &BranchName,
        create: Option<&WorktreeBase>,
    ) -> Result<SwitchOutcome, WorkspaceError> {
        let operation = self.locked_git(root).await?;
        let resolved = &operation.target;
        let _mutation = self.comparisons.mutating(
            root,
            resolved.project,
            super::comparison::Change::Repository,
        );
        let git = self.git()?;
        let outcome = git.switch(resolved.root.path(), branch, create).await?;
        if let Some(worktree) = resolved.worktree {
            let now = git.status(resolved.root.path()).await?.branch;
            self.branch_switched(worktree, now).await;
        }
        Ok(outcome)
    }

    /// `git.rename_branch`; a worktree's `created_branch` follows.
    ///
    /// # Errors
    /// As git's.
    pub(crate) async fn git_rename_branch(
        &self,
        root: WorkspaceRoot,
        from: Option<&BranchName>,
        to: &BranchName,
    ) -> Result<RenameOutcome, WorkspaceError> {
        let operation = self.locked_git(root).await?;
        let resolved = &operation.target;
        let _mutation = self.comparisons.mutating(
            root,
            resolved.project,
            super::comparison::Change::Repository,
        );
        let git = self.git()?;
        let dir = resolved.root.path();
        let from = match from {
            Some(from) => Some(from.clone()),
            None => git.status(dir).await?.branch,
        };
        let outcome = git.rename_branch(dir, from.as_ref(), to).await?;
        if let (RenameOutcome::Renamed { .. }, Some(from)) = (&outcome, &from) {
            self.branch_renamed(resolved.project, from, to).await;
        }
        Ok(outcome)
    }

    /// `git.stash`.
    ///
    /// # Errors
    /// As git's.
    pub(crate) async fn git_stash(
        &self,
        root: WorkspaceRoot,
        action: &StashAction,
    ) -> Result<StashOutcome, WorkspaceError> {
        let operation = self.locked_git(root).await?;
        let resolved = &operation.target;
        let _mutation =
            self.comparisons
                .mutating(root, resolved.project, super::comparison::Change::Working);
        Ok(self.git()?.stash(resolved.root.path(), action).await?)
    }
}

/// A scope whose base does not resolve is the caller's mistake.
fn scoped(
    resolved: Result<vessel_workspace::git::Resolved, GitError>,
) -> Result<vessel_workspace::git::Resolved, WorkspaceError> {
    match resolved {
        Err(GitError::Unexpected { what: "base" }) => Err(WorkspaceError::Invalid(
            "the scope's base does not name a commit".to_owned(),
        )),
        other => other.map_err(Into::into),
    }
}
