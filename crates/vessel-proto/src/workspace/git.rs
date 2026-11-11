//! Git: status, branches, history and what changes them (protocol 10,
//! ADR 0023). Expected refusals are typed outcomes ([`GitRefusal`]), not
//! errors, so no client reads git's own text.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::path::RelPath;
use super::text::{BranchName, CommitId, CommitMessage, RefName, RemoteName};
use super::worktree::{GitOperation, WorktreeBase};
use crate::id::{ProjectId, WorktreeId};

/// The folder a git or file call works in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceRoot {
    /// A project's folder.
    Project {
        /// Which.
        project: ProjectId,
    },
    /// A worktree's folder.
    Worktree {
        /// Which.
        worktree: WorktreeId,
    },
}

/// What a file is in the index or the working tree, against the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum FileState {
    /// No change on this side.
    Unmodified,
    /// Changed.
    Modified,
    /// New.
    Added,
    /// Removed.
    Deleted,
    /// Moved (`old_path` says from where).
    Renamed,
    /// Copied (`old_path` says from where).
    Copied,
    /// A file became a link or the other way round.
    TypeChanged,
    /// Not tracked (the working tree side).
    Untracked,
    /// In conflict: a merge left both sides.
    Conflicted,
    /// A state a newer node knows.
    #[serde(other)]
    Unknown,
}

/// A file with a change, as `git status` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StatusEntry {
    /// Its path.
    pub path: RelPath,
    /// Where it was, for a rename or a copy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_path: Option<RelPath>,
    /// Staged: the index against `HEAD`.
    pub index: FileState,
    /// Not staged: the working tree against the index.
    pub worktree: FileState,
}

/// How far a branch is from another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Distance {
    /// The other branch.
    pub name: RefName,
    /// Commits here that it does not have.
    pub ahead: u32,
    /// Commits it has that are not here.
    pub behind: u32,
}

/// A root's git status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GitStatus {
    /// The branch checked out; absent when detached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchName>,
    /// The commit checked out; absent before the first commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<CommitId>,
    /// Against the branch's upstream, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<Distance>,
    /// Against the base a worktree was made from, when that is a branch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<Distance>,
    /// An operation in progress.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<GitOperation>,
    /// Every changed, staged, untracked or conflicted file (not ignored
    /// ones), at most 5,000.
    pub files: Vec<StatusEntry>,
    /// Whether there were more.
    pub more: bool,
}

/// A branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Branch {
    /// Its name: `main`, or `origin/main` for a remote-tracking one.
    pub name: RefName,
    /// The remote, for a remote-tracking branch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<RemoteName>,
    /// Whether it is checked out in this root.
    pub current: bool,
    /// Whether it is the project's default branch.
    pub default: bool,
    /// Its commit.
    pub head: CommitId,
    /// When that commit was made.
    pub time: DateTime<Utc>,
    /// Against its upstream, for a local branch that has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<Distance>,
    /// The folder that has it checked out, when one does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<PathBuf>,
}

/// A root's branches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BranchList {
    /// Local branches first, then remote-tracking ones, each by name; at
    /// most 1,000.
    pub branches: Vec<Branch>,
    /// Whether there were more.
    pub more: bool,
}

/// A commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CommitInfo {
    /// Its id.
    pub id: CommitId,
    /// Its parents' ids.
    pub parents: Vec<CommitId>,
    /// Its author's name.
    pub author: String,
    /// When it was authored.
    pub time: DateTime<Utc>,
    /// Its message's first line.
    pub subject: String,
}

/// A page of history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CommitPage {
    /// Newest first.
    pub commits: Vec<CommitInfo>,
    /// Whether older ones follow.
    pub more: bool,
}

/// A stash entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StashEntry {
    /// Its position: 0 is the newest (`stash@{0}`).
    pub index: u32,
    /// Its commit, which names it when indexes move.
    pub commit: CommitId,
    /// Its message.
    pub message: String,
    /// When it was made.
    pub time: DateTime<Utc>,
}

/// A repository's stash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StashList {
    /// Newest first, at most 100.
    pub entries: Vec<StashEntry>,
    /// Whether there were more.
    pub more: bool,
}

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

/// Which files a call acts on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PathSet {
    /// Every changed file.
    All,
    /// These (at most 1,000).
    Paths {
        /// The paths.
        paths: Vec<RelPath>,
    },
}

/// What a commit takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CommitStage {
    /// What is staged.
    #[default]
    Staged,
    /// Every change to a tracked file (`git add -u` first).
    Tracked,
    /// Every change, untracked files included (`git add -A` first).
    Everything,
}

/// What to do with the stash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StashAction {
    /// Stash the local changes.
    Save {
        /// Its message.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<CommitMessage>,
        /// Untracked files too.
        #[serde(default)]
        untracked: bool,
    },
    /// Apply an entry and keep it.
    Apply {
        /// Which (`stash@{index}`).
        index: u32,
        /// The entry's commit, as listed: refused when the index moved.
        commit: CommitId,
    },
    /// Apply an entry and drop it when that worked.
    Pop {
        /// Which.
        index: u32,
        /// The entry's commit.
        commit: CommitId,
    },
    /// Drop an entry.
    Drop {
        /// Which.
        index: u32,
        /// The entry's commit.
        commit: CommitId,
    },
}

/// `git.status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetStatus {
    /// Where.
    pub root: WorkspaceRoot,
}

/// `git.branches`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListBranches {
    /// Where.
    pub root: WorkspaceRoot,
}

/// `git.log`: a page of history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadLog {
    /// Where.
    pub root: WorkspaceRoot,
    /// From this commit or ref; `HEAD` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<WorktreeBase>,
    /// Commits to skip from there.
    #[serde(default)]
    pub skip: u32,
    /// At most this many (at most 200, 50 when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// `git.stashes`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListStashes {
    /// Where.
    pub root: WorkspaceRoot,
}

/// `git.stage`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Stage {
    /// Where.
    pub root: WorkspaceRoot,
    /// Which files.
    pub paths: PathSet,
}

/// `git.unstage`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Unstage {
    /// Where.
    pub root: WorkspaceRoot,
    /// Which files.
    pub paths: PathSet,
}

/// `git.discard`: drop local changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Discard {
    /// Where.
    pub root: WorkspaceRoot,
    /// Which files.
    pub paths: PathSet,
    /// Delete the untracked ones among them too (never ignored files).
    #[serde(default)]
    pub untracked: bool,
}

/// `git.commit`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Commit {
    /// Where.
    pub root: WorkspaceRoot,
    /// The message.
    pub message: CommitMessage,
    /// What it takes.
    #[serde(default)]
    pub stage: CommitStage,
}

/// `git.push`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Push {
    /// Where.
    pub root: WorkspaceRoot,
    /// To this remote (`origin` when absent) instead of the upstream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<RemoteName>,
    /// Make the pushed branch the upstream.
    #[serde(default)]
    pub set_upstream: bool,
}

/// `git.fetch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Fetch {
    /// Where.
    pub root: WorkspaceRoot,
    /// This remote; the branch's, else `origin`, when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<RemoteName>,
}

/// `git.rename_branch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RenameBranch {
    /// Where.
    pub root: WorkspaceRoot,
    /// This branch; the one checked out when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<BranchName>,
    /// Its new name.
    pub to: BranchName,
}

/// `git.show`: a page of a file at a commit or ref (a diff's old side).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ShowFile {
    /// Where.
    pub root: WorkspaceRoot,
    /// At which commit or ref.
    pub at: WorktreeBase,
    /// Which file.
    pub path: RelPath,
    /// From this byte.
    #[serde(default)]
    pub offset: u64,
    /// At most this many bytes (at most 1 MiB, 256 KiB when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// `git.pull`: fast-forward from the upstream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Pull {
    /// Where.
    pub root: WorkspaceRoot,
}

/// `git.switch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Switch {
    /// Where.
    pub root: WorkspaceRoot,
    /// The branch.
    pub branch: BranchName,
    /// Create it from here; it must exist otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create: Option<WorktreeBase>,
}

/// `git.stash`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StashChanges {
    /// Where.
    pub root: WorkspaceRoot,
    /// What to do.
    pub action: StashAction,
}
