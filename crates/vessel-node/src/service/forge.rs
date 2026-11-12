//! Protocol 11's pull request calls (ADR 0029): routed to the node's
//! `forge` module, which decides. The guard has checked the scope, the
//! agents' rule and, for `pr.open`, `pr.merge` and `pr.feedback`, the
//! caller's ownership of the session.

use vessel_proto::forge::{
    ForgeStatus, GetForgeStatus, GetPullRequest, ListPullRequests, MergeOutcome, MergePullRequest,
    OpenOutcome, OpenPullRequest, PullRequestList, PullRequestView, SessionPullRequest,
    SetFeedback,
};
use vessel_proto::{ErrorCode, WireError};

use super::{Connection, NodeService};

/// Until the forge module answers them.
fn not_yet(method: &str) -> WireError {
    WireError::new(
        ErrorCode::Unsupported,
        format!("`{method}` is not served by this node yet"),
    )
}

#[expect(
    clippy::unused_async,
    reason = "the guard awaits every handler; these are served by the forge module once it is \
              built, and answer `unsupported` until then"
)]
impl NodeService {
    /// `forge.status`.
    pub(crate) async fn forge_status(
        &self,
        _: &Connection,
        _: GetForgeStatus,
    ) -> Result<ForgeStatus, WireError> {
        Err(not_yet("forge.status"))
    }

    /// `pr.list`.
    pub(crate) async fn list_pull_requests(
        &self,
        _: &Connection,
        _: ListPullRequests,
    ) -> Result<PullRequestList, WireError> {
        Err(not_yet("pr.list"))
    }

    /// `pr.get`.
    pub(crate) async fn get_pull_request(
        &self,
        _: &Connection,
        _: GetPullRequest,
    ) -> Result<PullRequestView, WireError> {
        Err(not_yet("pr.get"))
    }

    /// `pr.open`.
    pub(crate) async fn open_pull_request(
        &self,
        _: &Connection,
        _: OpenPullRequest,
    ) -> Result<OpenOutcome, WireError> {
        Err(not_yet("pr.open"))
    }

    /// `pr.merge`.
    pub(crate) async fn merge_pull_request(
        &self,
        _: &Connection,
        _: MergePullRequest,
    ) -> Result<MergeOutcome, WireError> {
        Err(not_yet("pr.merge"))
    }

    /// `pr.feedback`.
    pub(crate) async fn set_feedback(
        &self,
        _: &Connection,
        _: SetFeedback,
    ) -> Result<SessionPullRequest, WireError> {
        Err(not_yet("pr.feedback"))
    }
}
