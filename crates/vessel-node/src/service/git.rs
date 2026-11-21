//! Protocol 10's git calls (track B, ADR 0023): routing only; the work is
//! `crate::workspace`'s.

use vessel_proto::WireError;
use vessel_proto::workspace::{
    BranchList, Commit, CommitOutcome, CommitPage, DiffSummary, Discard, Fetch, FetchOutcome,
    FileDiff, FileRead, GetChanges, GetFileDiff, GetStatus, GitStatus, ListBranches, ListStashes,
    Pull, PullOutcome, Push, PushOutcome, ReadLog, RenameBranch, RenameOutcome, ShowFile, Stage,
    StashChanges, StashList, StashOutcome, Switch, SwitchOutcome, Unstage,
};

use super::projects::wire;
use super::{Connection, NodeService};
use crate::workspace::{IndexChange, Origin};

impl NodeService {
    /// `git.status`.
    pub(crate) async fn git_status(
        &self,
        connection: &Connection,
        request: GetStatus,
    ) -> Result<GitStatus, WireError> {
        self.workspaces
            .git_status(request.root, Origin::for_peer(connection.protocol()))
            .await
            .map(|status| self.known_origin(status))
            .map_err(|error| wire(&error))
    }

    /// `status` with what the node knows of its `origin`'s host (ADR 0123
    /// step B), when it carries one.
    fn known_origin(&self, mut status: GitStatus) -> GitStatus {
        if let Some(origin) = &mut status.origin {
            origin.kind = self.forges.kind_of(&origin.host);
        }
        status
    }

    /// `git.changes`.
    pub(crate) async fn git_changes(
        &self,
        _: &Connection,
        request: GetChanges,
    ) -> Result<DiffSummary, WireError> {
        self.workspaces
            .git_changes(request.root, &request.scope, request.ignore_whitespace)
            .await
            .map_err(|error| wire(&error))
    }

    /// `git.diff`.
    pub(crate) async fn git_diff(
        &self,
        _: &Connection,
        request: GetFileDiff,
    ) -> Result<FileDiff, WireError> {
        self.workspaces
            .git_diff(
                request.root,
                &request.scope,
                &request.path,
                request.ignore_whitespace,
                request.from_hunk.unwrap_or(0),
            )
            .await
            .map_err(|error| wire(&error))
    }

    /// `git.show`.
    pub(crate) async fn git_show(
        &self,
        _: &Connection,
        request: ShowFile,
    ) -> Result<FileRead, WireError> {
        self.workspaces
            .git_show(
                request.root,
                &request.at,
                &request.path,
                request.offset,
                request.limit,
            )
            .await
            .map_err(|error| wire(&error))
    }

    /// `git.branches`.
    pub(crate) async fn git_branches(
        &self,
        _: &Connection,
        request: ListBranches,
    ) -> Result<BranchList, WireError> {
        self.workspaces
            .git_branches(request.root)
            .await
            .map_err(|error| wire(&error))
    }

    /// `git.log`.
    pub(crate) async fn git_log(
        &self,
        _: &Connection,
        request: ReadLog,
    ) -> Result<CommitPage, WireError> {
        self.workspaces
            .git_log(
                request.root,
                request.from.as_ref(),
                request.skip,
                request.limit,
            )
            .await
            .map_err(|error| wire(&error))
    }

    /// `git.stashes`.
    pub(crate) async fn git_stashes(
        &self,
        _: &Connection,
        request: ListStashes,
    ) -> Result<StashList, WireError> {
        self.workspaces
            .git_stashes(request.root)
            .await
            .map_err(|error| wire(&error))
    }

    /// `git.stage`.
    pub(crate) async fn git_stage(
        &self,
        connection: &Connection,
        request: Stage,
    ) -> Result<GitStatus, WireError> {
        self.workspaces
            .git_index(
                request.root,
                &request.paths,
                IndexChange::Stage,
                Origin::for_peer(connection.protocol()),
            )
            .await
            .map(|status| self.known_origin(status))
            .map_err(|error| wire(&error))
    }

    /// `git.unstage`.
    pub(crate) async fn git_unstage(
        &self,
        connection: &Connection,
        request: Unstage,
    ) -> Result<GitStatus, WireError> {
        self.workspaces
            .git_index(
                request.root,
                &request.paths,
                IndexChange::Unstage,
                Origin::for_peer(connection.protocol()),
            )
            .await
            .map(|status| self.known_origin(status))
            .map_err(|error| wire(&error))
    }

    /// `git.discard`.
    pub(crate) async fn git_discard(
        &self,
        connection: &Connection,
        request: Discard,
    ) -> Result<GitStatus, WireError> {
        self.workspaces
            .git_index(
                request.root,
                &request.paths,
                IndexChange::Discard {
                    untracked: request.untracked,
                },
                Origin::for_peer(connection.protocol()),
            )
            .await
            .map(|status| self.known_origin(status))
            .map_err(|error| wire(&error))
    }

    /// `git.commit`.
    pub(crate) async fn git_commit(
        &self,
        _: &Connection,
        request: Commit,
    ) -> Result<CommitOutcome, WireError> {
        self.workspaces
            .git_commit(request.root, &request.message, request.stage)
            .await
            .map_err(|error| wire(&error))
    }

    /// `git.fetch`.
    pub(crate) async fn git_fetch(
        &self,
        _: &Connection,
        request: Fetch,
    ) -> Result<FetchOutcome, WireError> {
        self.workspaces
            .git_fetch(request.root, request.remote.as_ref())
            .await
            .map_err(|error| wire(&error))
    }

    /// `git.push`.
    pub(crate) async fn git_push(
        &self,
        _: &Connection,
        request: Push,
    ) -> Result<PushOutcome, WireError> {
        self.workspaces
            .git_push(request.root, request.remote.as_ref(), request.set_upstream)
            .await
            .map_err(|error| wire(&error))
    }

    /// `git.pull`.
    pub(crate) async fn git_pull(
        &self,
        _: &Connection,
        request: Pull,
    ) -> Result<PullOutcome, WireError> {
        self.workspaces
            .git_pull(request.root)
            .await
            .map_err(|error| wire(&error))
    }

    /// `git.switch`.
    pub(crate) async fn git_switch(
        &self,
        _: &Connection,
        request: Switch,
    ) -> Result<SwitchOutcome, WireError> {
        self.workspaces
            .git_switch(request.root, &request.branch, request.create.as_ref())
            .await
            .map_err(|error| wire(&error))
    }

    /// `git.rename_branch`.
    pub(crate) async fn git_rename_branch(
        &self,
        _: &Connection,
        request: RenameBranch,
    ) -> Result<RenameOutcome, WireError> {
        self.workspaces
            .git_rename_branch(request.root, request.from.as_ref(), &request.to)
            .await
            .map_err(|error| wire(&error))
    }

    /// `git.stash`.
    pub(crate) async fn git_stash(
        &self,
        _: &Connection,
        request: StashChanges,
    ) -> Result<StashOutcome, WireError> {
        self.workspaces
            .git_stash(request.root, &request.action)
            .await
            .map_err(|error| wire(&error))
    }
}
