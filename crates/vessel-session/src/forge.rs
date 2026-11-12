//! A session's pull request, as its node gives it (protocol 11, ADR
//! 0029): what `SessionInfo.pull_request` shows, and the facts the board's
//! reducer reads.
//!
//! [Decision Log]
//! - Purpose: the board's three right-hand lanes are filled by the same
//!   reducer that places every session (ADR 0019): the session's actor
//!   derives its board, so the pull request's facts must reach the actor,
//!   not a second place that decides a lane.
//! - Choice: one `watch` per session, the node the sender, the actor the
//!   receiver: the latest view wins and nothing queues (a pull request read
//!   twice before the actor looks is read once). The actor keeps the facts
//!   in its activity tracker, so every later record's board has them, and
//!   drops them when the session ends, before the final board, so an ended
//!   session is placed as the store places a stored one (ADR 0026).
//! - The view carries the reducer's facts built by the node from the whole
//!   pull request ([`board::PullRequestFacts::from_pull`]): the summary the
//!   info shows lacks people's reviews.

use std::sync::Arc;

use vessel_proto::forge::SessionPullRequest;

use crate::board::{Policy, PullRequestFacts};

/// What a node knows of a session's pull request now.
#[derive(Debug, Clone, PartialEq)]
pub struct ForgeView {
    /// What `SessionInfo.pull_request` shows.
    pub info: SessionPullRequest,
    /// The reducer's facts: `None` while the node cannot see the session's
    /// pull requests (`PullRequests::Unseen`); `Some` once it read them,
    /// empty when there is none.
    pub seen: Option<Arc<[PullRequestFacts]>>,
    /// The loops the node drives for the session (its feedback switch).
    pub policy: Policy,
}
