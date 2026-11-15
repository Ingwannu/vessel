//! Pull requests on a forge (protocol 11, milestone 11, track B2, ADR
//! 0029): what a session's pull request is, whether the node can see it,
//! and the calls that read, open and merge one.
//! ADR 0061's project-page values are also defined here, without a
//! registered project-list call until its protocol admission is ready.
//!
//! [Decision Log]
//! - Purpose: agent-orchestrator, GOAT and Paseo all follow a session's
//!   pull request, and Vessel's board has three lanes (ADR 0019) that only
//!   a pull request's facts fill.
//! - Constraint: one wire for every forge (GitHub now; GitLab and Gitea
//!   behind the same trait later, `vessel-forge`), where their numbers,
//!   checks and verdicts differ in name, not in kind.
//! - Choice: a pull request is reached through its session (the session's
//!   branch in its project's repository), never by a free URL a client
//!   names: the node only ever talks to the repository a session's own
//!   remote names. Closed enums with `unknown` wherever a forge may say
//!   more later. A summary rides in `SessionInfo`; the rest is `pr.get`'s.
//! - Older peers: a protocol 8 to 10 client ignores
//!   `SessionInfo.pull_request` and already draws every lane a pull
//!   request places a session in; no event is added.

mod access;
mod calls;
mod names;
mod project;
mod pull;

pub use access::{
    ForgeAccess, ForgeStatus, GhState, HostStatus, PullRequestList, PullRequestView, RefusedReason,
    SessionPullEntry, SessionPullRequest, TokenSource,
};
pub use calls::{
    GetForgeStatus, GetPullRequest, ListPullRequests, MergeOutcome, MergePullRequest, MergeRefusal,
    OpenOutcome, OpenPullRequest, OpenRefusal, SetFeedback,
};
pub use names::{
    CheckName, ForgeHost, ForgeLogin, ForgeUrl, InvalidForgeValue, PullBody, PullNumber, PullTitle,
    RepoPath, is_format,
};
pub use project::{
    InvalidOpenPullPage, InvalidPullCursor, OpenPullPage, PROJECT_PULL_REQUESTS, ProjectPullCursor,
    PullCursor,
};
pub use pull::{
    CheckRun, CheckState, Checks, ChecksState, ForgeKind, MergeMethod, Mergeable, PullRequest,
    PullState, PullSummary, RepoRef, ReviewDecision, Reviews,
};

#[cfg(test)]
mod tests;
