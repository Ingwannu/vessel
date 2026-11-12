//! What a change answers: done, with what it did, or refused with a
//! typed reason ([`GitRefusal`]) a client can act on without reading
//! git's own text.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::state::{CommitInfo, StashList};
use crate::workspace::path::RelPath;
use crate::workspace::text::{BranchName, CommitId, ConfigKey, RemoteName};
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
    /// The repository's own configuration sets something the node's git
    /// does not run with (protocol 11, ADR 0030): a program for this run
    /// that turning off would also turn off the user's own global one, or
    /// a working tree moved elsewhere. Never the value.
    RepositoryConfig {
        /// The key, as git lists it; absent when the whole configuration
        /// was refused (`too_large`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        key: Option<ConfigKey>,
        /// Why.
        reason: RepositoryConfigReason,
    },
    /// A refusal a newer node knows and this client does not.
    #[serde(other)]
    Unknown,
}

/// Why a git run was refused for its repository's own configuration
/// (protocol 11, ADR 0030).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum RepositoryConfigReason {
    /// It names a program this run would start (an `ssh`, a credential
    /// helper, a signing program, a remote's upload or receive program).
    Program,
    /// It moves the working tree out of the folder.
    Layout,
    /// It could send the run's credential elsewhere (a proxy, a weakened
    /// TLS check, a resolver, an extra header).
    Credential,
    /// A driver whose name cannot be turned off on git's command line.
    Unnamable,
    /// More drivers than one run turns off.
    TooMany,
    /// The configuration is larger than the node reads before a run.
    TooLarge,
    /// A reason a newer node knows and this client does not.
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
