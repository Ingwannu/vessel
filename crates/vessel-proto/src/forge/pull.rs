//! A pull request as the node reads it from a forge (protocol 11, ADR
//! 0029): its state, its checks, its reviews and whether it can merge,
//! typed as agent-orchestrator's board facts need them, and named the way
//! GitLab and Gitea can fill too.
//!
//! [Decision Log]
//! - Purpose: the board's three right-hand lanes (ADR 0019) and a pull
//!   request's card need the same facts every reference reads (draft,
//!   state, checks, review decision, mergeability, head and base).
//! - Choice: closed enums with `unknown` for every value a forge reports,
//!   matching agent-orchestrator's mapping (the plan's "The facts, and the
//!   board"); a summary small enough for `SessionInfo`, which every client
//!   lists every two seconds, and the whole pull request for `pr.get`.
//! - Must stay true: nothing here is a forge's own vocabulary (no
//!   `mergeStateStatus` string): the provider maps it once, where it reads
//!   it (`vessel-forge`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::names::{CheckName, ForgeHost, ForgeLogin, ForgeUrl, PullNumber, PullTitle, RepoPath};
use crate::workspace::{BranchName, CommitId};

/// Which kind of forge a repository is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ForgeKind {
    /// GitHub, or GitHub Enterprise.
    Github,
    /// A kind a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// A repository on a forge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RepoRef {
    /// Which kind of forge.
    pub kind: ForgeKind,
    /// Its host, as the remote names it.
    pub host: ForgeHost,
    /// `owner/name` (or GitLab's nested groups).
    pub path: RepoPath,
}

/// Where a pull request is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PullState {
    /// Open (a draft too; `draft` says which).
    Open,
    /// Merged.
    Merged,
    /// Closed without merging.
    Closed,
    /// A state a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// One check's result (agent-orchestrator's table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    /// Queued or running.
    Pending,
    /// Passed (or neutral).
    Passed,
    /// Failed, timed out, or needs action.
    Failed,
    /// Cancelled.
    Cancelled,
    /// Skipped or stale.
    Skipped,
    /// A result a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// All of a head commit's checks together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ChecksState {
    /// No check was reported.
    None,
    /// One is still running, none failed.
    Pending,
    /// Every one passed.
    Passing,
    /// One failed.
    Failing,
    /// One was cancelled and nothing runs or failed.
    Cancelled,
    /// Not known (more checks than one read takes, or not read yet).
    #[serde(other)]
    Unknown,
}

/// One check, as a card names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CheckRun {
    /// Its name.
    pub name: CheckName,
    /// Its result.
    pub state: CheckState,
    /// Its page on the forge, when it gave an `https` one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<ForgeUrl>,
}

/// A head commit's checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Checks {
    /// All of them together.
    pub state: ChecksState,
    /// How many passed (neutral and skipped included).
    pub passed: u32,
    /// How many failed.
    pub failed: u32,
    /// How many run or wait.
    pub pending: u32,
    /// More were reported than one read takes (100).
    pub truncated: bool,
    /// The failing ones, at most [`Checks::MAX_FAILING`].
    pub failing: Vec<CheckRun>,
}

impl Checks {
    /// The most failing checks named.
    pub const MAX_FAILING: usize = 20;

    /// No checks known.
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            state: ChecksState::Unknown,
            passed: 0,
            failed: 0,
            pending: 0,
            truncated: false,
            failing: Vec::new(),
        }
    }
}

/// The forge's review verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    /// Approved.
    Approved,
    /// Changes were requested.
    ChangesRequested,
    /// A review is required before it can merge.
    ReviewRequired,
    /// No verdict.
    None,
    /// A verdict a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// Whether it can be merged now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Mergeable {
    /// It can.
    Mergeable,
    /// It conflicts with its base.
    Conflicting,
    /// A rule blocks it (a review, a check, a draft).
    Blocked,
    /// It can, but a check is not green.
    Unstable,
    /// Its base moved on; it needs updating first.
    Behind,
    /// Not known (the forge is still computing it).
    #[serde(other)]
    Unknown,
}

/// People's reviews on the current head, the latest per person, bots
/// left out (agent-orchestrator's `ExternalReviewFacts`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Reviews {
    /// A person's approval stands.
    pub approved: bool,
    /// A person requested changes.
    pub changes_requested: bool,
    /// An unresolved thread holds a person's comment.
    pub commented: bool,
    /// Unresolved review threads.
    pub unresolved: u32,
}

/// How a pull request is merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MergeMethod {
    /// A merge commit.
    Merge,
    /// One commit of all its changes.
    Squash,
    /// Its commits replayed on the base.
    Rebase,
}

/// What a card needs of a pull request; carried in `SessionInfo`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PullSummary {
    /// Its repository.
    pub repo: RepoRef,
    /// Its number there.
    pub number: PullNumber,
    /// Its page.
    pub url: ForgeUrl,
    /// Its title.
    pub title: PullTitle,
    /// Open, merged or closed.
    pub state: PullState,
    /// A draft.
    pub draft: bool,
    /// The commit its head is at.
    pub head_commit: CommitId,
    /// Its checks, together.
    pub checks: ChecksState,
    /// The forge's review verdict.
    pub review: ReviewDecision,
    /// Whether it can merge now.
    pub mergeable: Mergeable,
    /// When it last changed on the forge.
    pub updated_at: DateTime<Utc>,
}

/// A pull request, whole: what `pr.get` answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PullRequest {
    /// What a card shows.
    pub summary: PullSummary,
    /// Who opened it, when the forge named a login.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<ForgeLogin>,
    /// Its head branch.
    pub head_branch: BranchName,
    /// The branch it merges into.
    pub base: BranchName,
    /// Its head commit's checks.
    pub checks: Checks,
    /// People's reviews.
    pub reviews: Reviews,
    /// When it was opened.
    pub created_at: DateTime<Utc>,
    /// When it was merged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merged_at: Option<DateTime<Utc>>,
    /// When it was closed (merged or not).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,
    /// The methods its repository allows.
    pub merge_methods: Vec<MergeMethod>,
    /// When this node last read it.
    pub read_at: DateTime<Utc>,
}
