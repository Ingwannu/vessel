//! Whether a node can see a session's pull request, and why not when it
//! cannot (protocol 11, ADR 0029); what a session carries of it; and how
//! the node is signed in to each forge.
//!
//! [Decision Log]
//! - Purpose: "no pull request" and "cannot tell" are different claims
//!   (ADR 0019's `Unseen`), and a person who sees no pull request on a card
//!   needs to know which, and what to do: sign `gh` in, add a remote, wait
//!   for a rate limit.
//! - Choice: one closed enum, [`ForgeAccess`], every reason a value of it,
//!   a host and a time where they help; never a token, and never a forge's
//!   message where a typed reason fits.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::names::{ForgeHost, ForgeLogin, ForgeUrl};
use super::pull::{ForgeKind, PullRequest, PullSummary};
use crate::id::SessionId;

/// Why a forge refused the node's token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RefusedReason {
    /// The token is not good (401).
    Unauthorized,
    /// The repository is not one the login can see (404, 410, 451).
    NoAccess,
    /// The organisation asks for single sign-on for this token (403).
    Sso {
        /// Where to authorise it, when the forge said.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<ForgeUrl>,
    },
    /// Another 403: the token lacks a permission.
    Forbidden,
    /// A reason a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// What looking for the machine's `gh` found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum GhState {
    /// There is no `gh` on the node's search path.
    Missing,
    /// `gh` has no token for that host.
    SignedOut,
    /// `gh` failed or did not answer in time.
    Failed,
    /// A state a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// Where a node's token for a host came from (never the token).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TokenSource {
    /// The node's environment (`GH_TOKEN`, `GITHUB_TOKEN`, or the
    /// enterprise ones for `GH_HOST`).
    Environment,
    /// The machine's `gh` (`gh auth token`).
    Gh,
    /// A source a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// Whether the node can see a session's pull request, and why not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "access", rename_all = "snake_case")]
pub enum ForgeAccess {
    /// It reads it.
    Ready {
        /// Who the node is signed in as, when the forge said.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        login: Option<ForgeLogin>,
    },
    /// The session has no project or worktree, so no repository.
    NotPlaced,
    /// The session's folder is on no branch.
    NoBranch,
    /// The session is on its project's default branch, where no pull
    /// request starts.
    OnBase,
    /// The branch's remote (or `origin`) does not exist.
    NoRemote,
    /// No provider for that host on this node, or no token for it.
    NotSupported {
        /// The host.
        host: ForgeHost,
    },
    /// No token: `gh` is missing or signed out, and none was given.
    SignedOut {
        /// The host.
        host: ForgeHost,
        /// What `gh` said.
        gh: GhState,
    },
    /// The forge refused the token.
    Refused {
        /// The host.
        host: ForgeHost,
        /// Why.
        reason: RefusedReason,
        /// Who the node is signed in as, when known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        login: Option<ForgeLogin>,
    },
    /// The forge's rate limit holds every read until `until`.
    RateLimited {
        /// The host.
        host: ForgeHost,
        /// When reading starts again.
        until: DateTime<Utc>,
    },
    /// The forge could not be reached; the next try is at `retry_at`.
    Unreachable {
        /// The host.
        host: ForgeHost,
        /// When the node tries again.
        retry_at: DateTime<Utc>,
    },
    /// The node watches as many as it allows.
    Limit,
    /// The session is not live, so nothing is watched for it.
    Ended,
    /// A reason a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// What a session carries of its pull request (`SessionInfo.pull_request`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SessionPullRequest {
    /// Whether the node can see it.
    pub access: ForgeAccess,
    /// The last one read, kept while access fails later.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<PullSummary>,
    /// Whether its failing checks and reviews are sent to the session.
    pub feedback: bool,
}

/// `pr.get`'s answer: the pull request whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PullRequestView {
    /// Whether the node can see it.
    pub access: ForgeAccess,
    /// The last one read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<Box<PullRequest>>,
    /// Whether its failing checks and reviews are sent to the session.
    pub feedback: bool,
}

/// How the node is signed in to one host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HostStatus {
    /// The host.
    pub host: ForgeHost,
    /// Its kind.
    pub kind: ForgeKind,
    /// What signing in found.
    pub access: ForgeAccess,
    /// Where the token came from, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<TokenSource>,
}

/// `forge.status`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ForgeStatus {
    /// Every host the node watched or was asked about, at most
    /// [`ForgeStatus::MAX_HOSTS`].
    pub hosts: Vec<HostStatus>,
    /// Pull requests watched now.
    pub watched: u32,
    /// The most it watches.
    pub limit: u32,
}

impl ForgeStatus {
    /// The most hosts listed: a remote an agent controls could otherwise
    /// add hosts without end.
    pub const MAX_HOSTS: usize = 16;
}

/// One watched session's pull request (`pr.list`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SessionPullEntry {
    /// The session.
    pub session: SessionId,
    /// Its pull request.
    pub pull_request: SessionPullRequest,
}

/// `pr.list`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PullRequestList {
    /// Every watched (live) session's, at most the watch limit.
    pub entries: Vec<SessionPullEntry>,
}
