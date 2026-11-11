//! Protocol 10's worktree calls (track B, ADR 0024): routing only; the work is `crate::workspace`'s.

#![expect(
    clippy::unused_async,
    reason = "the calls answer `unsupported` until their slice is built; each becomes a real call"
)]

use vessel_proto::{Ack, ErrorCode, WireError};

use super::{Connection, NodeService};

impl NodeService {
    /// `worktree.list`.
    pub(crate) async fn list_worktrees(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::ListWorktrees,
    ) -> Result<vessel_proto::workspace::WorktreeList, WireError> {
        Err(not_built("worktree.list"))
    }

    /// `worktree.create`.
    pub(crate) async fn create_worktree(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::CreateWorktree,
    ) -> Result<vessel_proto::workspace::CreateWorktreeOutcome, WireError> {
        Err(not_built("worktree.create"))
    }

    /// `worktree.output`.
    pub(crate) async fn setup_output(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::ReadSetupOutput,
    ) -> Result<vessel_proto::workspace::SetupOutput, WireError> {
        Err(not_built("worktree.output"))
    }

    /// `worktree.setup`.
    pub(crate) async fn run_setup(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::RunSetup,
    ) -> Result<vessel_proto::workspace::WorktreeInfo, WireError> {
        Err(not_built("worktree.setup"))
    }

    /// `worktree.remove`.
    pub(crate) async fn remove_worktree(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::RemoveWorktree,
    ) -> Result<vessel_proto::workspace::RemoveWorktreeOutcome, WireError> {
        Err(not_built("worktree.remove"))
    }

    /// `worktree.snapshots`.
    pub(crate) async fn list_snapshots(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::ListSnapshots,
    ) -> Result<vessel_proto::workspace::SnapshotList, WireError> {
        Err(not_built("worktree.snapshots"))
    }

    /// `worktree.drop_snapshot`.
    pub(crate) async fn drop_snapshot(
        &self,
        _: &Connection,
        _request: vessel_proto::workspace::DropSnapshot,
    ) -> Result<Ack, WireError> {
        Err(not_built("worktree.drop_snapshot"))
    }
}

fn not_built(method: &str) -> WireError {
    WireError::new(
        ErrorCode::Unsupported,
        format!("`{method}` is not built on this node yet"),
    )
}
