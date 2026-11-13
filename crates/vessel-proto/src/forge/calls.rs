//! The forge's calls (protocol 11, ADR 0029): what each asks and what it
//! answers. Which principal may call each is the node's (`guard`); what
//! each does is the plan's "Track B2: contract".

use serde::{Deserialize, Serialize};

use super::access::ForgeAccess;
use super::names::{PullBody, PullNumber, PullTitle};
use super::pull::{MergeMethod, PullRequest, PullState};
use crate::id::SessionId;
use crate::workspace::{BranchName, CommitId, GitRefusal};

/// `forge.status`: how the node is signed in to each forge.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetForgeStatus {}

/// `pr.list`: every watched session's pull request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListPullRequests {}

/// `pr.get`: a session's pull request, whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetPullRequest {
    /// The session.
    pub session: SessionId,
    /// Read it from the forge now (at most once per 10 s per session;
    /// otherwise the watcher's copy).
    #[serde(default)]
    pub refresh: bool,
}

/// `pr.open`: open a pull request from the branch a session's folder is
/// on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OpenPullRequest {
    /// The session.
    pub session: SessionId,
    /// Its title.
    pub title: PullTitle,
    /// Its description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<PullBody>,
    /// The branch it merges into; the project's default branch when
    /// absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<BranchName>,
    /// Open it as a draft.
    #[serde(default)]
    pub draft: bool,
}

/// Why `pr.open` made nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpenRefusal {
    /// An agent's call, whose branch is not on the remote at the
    /// folder's `HEAD`: the node does not push for an agent.
    NotPushed,
    /// The branch has no commit its base has not.
    NoCommits,
    /// Pushing the branch was refused.
    Push {
        /// Why.
        refusal: GitRefusal,
    },
    /// The node cannot reach the forge for it.
    Access {
        /// Why.
        access: ForgeAccess,
    },
    /// The forge refused, in its own words.
    Rejected {
        /// Its message, at most 2 KiB.
        message: String,
    },
    /// A refusal a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// `pr.open`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum OpenOutcome {
    /// Opened, and watched.
    Opened {
        /// It.
        pull_request: Box<PullRequest>,
    },
    /// An open one has that head already; nothing new was made.
    Exists {
        /// It.
        pull_request: Box<PullRequest>,
    },
    /// Nothing was made.
    Refused {
        /// Why.
        refusal: OpenRefusal,
    },
    /// An outcome a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// `pr.merge`: merge the session's pull request, as the caller saw it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MergePullRequest {
    /// The session.
    pub session: SessionId,
    /// The pull request the caller looked at.
    pub number: PullNumber,
    /// The method.
    pub method: MergeMethod,
    /// The head the caller looked at, in full (40 or 64 digits): a push
    /// since refuses the merge.
    pub head: CommitId,
}

/// Why `pr.merge` merged nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MergeRefusal {
    /// The session has no pull request.
    NoPullRequest,
    /// The session's pull request is another one.
    OtherPullRequest {
        /// Its number.
        number: PullNumber,
    },
    /// It is not open.
    NotOpen {
        /// What it is.
        state: PullState,
    },
    /// It is a draft.
    Draft,
    /// Its head moved since the caller looked.
    HeadMoved {
        /// The head now.
        head: CommitId,
    },
    /// The forge says it cannot merge now.
    NotMergeable {
        /// The forge's words, when it gave some.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// The repository does not allow that method.
    MethodNotAllowed,
    /// The repository merges through a queue, which this node does not
    /// drive.
    MergeQueue,
    /// This forge cannot pin the head for that method.
    CannotPin,
    /// The node cannot reach the forge for it.
    Access {
        /// Why.
        access: ForgeAccess,
    },
    /// The forge refused, in its own words.
    Rejected {
        /// Its message, at most 2 KiB.
        message: String,
    },
    /// A refusal a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// `pr.merge`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum MergeOutcome {
    /// Merged.
    Merged {
        /// The commit the merge made on the base.
        commit: CommitId,
    },
    /// Nothing was merged.
    Refused {
        /// Why.
        refusal: MergeRefusal,
    },
    /// An outcome a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// `pr.feedback`: send the pull request's failing checks and reviews to
/// the session, or stop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetFeedback {
    /// The session.
    pub session: SessionId,
    /// On or off.
    pub enabled: bool,
}
