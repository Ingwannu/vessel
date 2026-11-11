//! Protocol 10's git calls (track B, ADR 0023): routing only; the work is `crate::workspace`'s.

#![expect(
    clippy::unused_async,
    reason = "the calls answer `unsupported` until their slice is built; each becomes a real call"
)]

use vessel_proto::{ErrorCode, WireError};

use super::{Connection, NodeService};

impl NodeService {
    /// `git.status`.
    pub(crate) async fn git_status(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::GetStatus,
    ) -> Result<vessel_proto::workspace::GitStatus, WireError> {
        Err(not_built("git.status"))
    }

    /// `git.changes`.
    pub(crate) async fn git_changes(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::GetChanges,
    ) -> Result<vessel_proto::workspace::DiffSummary, WireError> {
        Err(not_built("git.changes"))
    }

    /// `git.diff`.
    pub(crate) async fn git_diff(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::GetFileDiff,
    ) -> Result<vessel_proto::workspace::FileDiff, WireError> {
        Err(not_built("git.diff"))
    }

    /// `git.show`.
    pub(crate) async fn git_show(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::ShowFile,
    ) -> Result<vessel_proto::workspace::FileRead, WireError> {
        Err(not_built("git.show"))
    }

    /// `git.branches`.
    pub(crate) async fn git_branches(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::ListBranches,
    ) -> Result<vessel_proto::workspace::BranchList, WireError> {
        Err(not_built("git.branches"))
    }

    /// `git.log`.
    pub(crate) async fn git_log(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::ReadLog,
    ) -> Result<vessel_proto::workspace::CommitPage, WireError> {
        Err(not_built("git.log"))
    }

    /// `git.stashes`.
    pub(crate) async fn git_stashes(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::ListStashes,
    ) -> Result<vessel_proto::workspace::StashList, WireError> {
        Err(not_built("git.stashes"))
    }

    /// `git.stage`.
    pub(crate) async fn git_stage(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::Stage,
    ) -> Result<vessel_proto::workspace::GitStatus, WireError> {
        Err(not_built("git.stage"))
    }

    /// `git.unstage`.
    pub(crate) async fn git_unstage(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::Unstage,
    ) -> Result<vessel_proto::workspace::GitStatus, WireError> {
        Err(not_built("git.unstage"))
    }

    /// `git.discard`.
    pub(crate) async fn git_discard(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::Discard,
    ) -> Result<vessel_proto::workspace::GitStatus, WireError> {
        Err(not_built("git.discard"))
    }

    /// `git.commit`.
    pub(crate) async fn git_commit(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::Commit,
    ) -> Result<vessel_proto::workspace::CommitOutcome, WireError> {
        Err(not_built("git.commit"))
    }

    /// `git.fetch`.
    pub(crate) async fn git_fetch(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::Fetch,
    ) -> Result<vessel_proto::workspace::FetchOutcome, WireError> {
        Err(not_built("git.fetch"))
    }

    /// `git.push`.
    pub(crate) async fn git_push(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::Push,
    ) -> Result<vessel_proto::workspace::PushOutcome, WireError> {
        Err(not_built("git.push"))
    }

    /// `git.pull`.
    pub(crate) async fn git_pull(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::Pull,
    ) -> Result<vessel_proto::workspace::PullOutcome, WireError> {
        Err(not_built("git.pull"))
    }

    /// `git.switch`.
    pub(crate) async fn git_switch(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::Switch,
    ) -> Result<vessel_proto::workspace::SwitchOutcome, WireError> {
        Err(not_built("git.switch"))
    }

    /// `git.rename_branch`.
    pub(crate) async fn git_rename_branch(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::RenameBranch,
    ) -> Result<vessel_proto::workspace::RenameOutcome, WireError> {
        Err(not_built("git.rename_branch"))
    }

    /// `git.stash`.
    pub(crate) async fn git_stash(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::StashChanges,
    ) -> Result<vessel_proto::workspace::StashOutcome, WireError> {
        Err(not_built("git.stash"))
    }
}

fn not_built(method: &str) -> WireError {
    WireError::new(
        ErrorCode::Unsupported,
        format!("`{method}` is not built on this node yet"),
    )
}
