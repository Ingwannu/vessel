//! A repository as the git reads answer it: a root's status, its
//! branches, a page of history and the stash. Nothing here changes a
//! repository.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::origin::OriginRepo;
use crate::workspace::path::RelPath;
use crate::workspace::text::{BranchName, CommitId, RefName, RemoteName};
use crate::workspace::worktree::GitOperation;

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
    /// Where `origin` keeps the repository (ADR 0123 step B); absent when
    /// there is no `origin`, its address names no forge, or the peer is
    /// below `ORIGIN_LINK_PROTOCOL`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<OriginRepo>,
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
