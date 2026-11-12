//! What a change answers: done, with what it did, or refused with a
//! typed reason ([`GitRefusal`]) a client can act on without reading
//! git's own text.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::state::{CommitInfo, StashList};
use crate::workspace::path::RelPath;
use crate::workspace::text::{BranchName, CommitId, RemoteName};
use crate::workspace::worktree::GitOperation;

/// Why git refused a change: what a client can act on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GitRefusal {
    /// The remote wants a credential git has none for (prompts are off).
    NeedsCredential,
    /// It ran past its time limit and was stopped.
    TimedOut,
    /// The remote has commits this branch does not: pull first.
    NonFastForward,
    /// Both sides have commits: a fast-forward is not possible.
    Diverged,
    /// Local changes would be overwritten.
    WouldOverwrite {
        /// The files, at most 100.
        paths: Vec<RelPath>,
    },
    /// Applying left conflicts to resolve.
    Conflicts {
        /// The files, at most 100.
        paths: Vec<RelPath>,
    },
    /// Nothing is staged (or changed, for `everything`).
    NothingToCommit,
    /// The repository has no author name or email configured.
    NoIdentity,
    /// A hook refused.
    HookFailed {
        /// Its last lines.
        output: String,
    },
    /// The branch has no upstream and none was asked for.
    NoUpstream,
    /// No branch is checked out.
    Detached,
    /// The branch is checked out in another folder.
    CheckedOutElsewhere {
        /// That folder.
        path: PathBuf,
    },
    /// No local change to stash.
    NothingToStash,
    /// The stash entry at that index is not the commit named (the stash is
    /// shared by every worktree, so indexes move).
    StashMoved,
    /// The branch exists already.
    BranchExists,
    /// No such branch, ref or commit.
    NoSuchRef,
    /// An operation is in progress.
    Operation {
        /// Which.
        operation: GitOperation,
    },
    /// The remote refused, or another failure git explained.
    RemoteFailed {
        /// git's last lines, credentials removed.
        message: String,
    },
    /// A refusal a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// `git.commit`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum CommitOutcome {
    /// Done.
    Committed {
        /// The new commit.
        commit: CommitInfo,
    },
    /// Refused.
    Refused {
        /// Why.
        refusal: GitRefusal,
    },
}

/// `git.push`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PushOutcome {
    /// Done.
    Pushed {
        /// Where to.
        remote: RemoteName,
        /// Which branch.
        branch: BranchName,
        /// Whether the push set the branch's upstream.
        upstream_set: bool,
    },
    /// Refused.
    Refused {
        /// Why.
        refusal: GitRefusal,
    },
}

/// `git.pull`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PullOutcome {
    /// Fast-forwarded.
    Pulled {
        /// The commit before.
        from: CommitId,
        /// The commit now.
        to: CommitId,
    },
    /// Nothing new.
    UpToDate,
    /// Refused.
    Refused {
        /// Why.
        refusal: GitRefusal,
    },
}

/// `git.fetch`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum FetchOutcome {
    /// Done.
    Fetched,
    /// Refused.
    Refused {
        /// Why.
        refusal: GitRefusal,
    },
}

/// `git.rename_branch`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RenameOutcome {
    /// Done.
    Renamed {
        /// Its name now.
        branch: BranchName,
    },
    /// Refused.
    Refused {
        /// Why.
        refusal: GitRefusal,
    },
}

/// `git.switch`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum SwitchOutcome {
    /// The branch is checked out.
    Switched {
        /// Which.
        branch: BranchName,
    },
    /// Refused.
    Refused {
        /// Why.
        refusal: GitRefusal,
    },
}

/// `git.stash`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum StashOutcome {
    /// Done.
    Done {
        /// The stash now.
        stashes: StashList,
    },
    /// Refused.
    Refused {
        /// Why.
        refusal: GitRefusal,
    },
}
