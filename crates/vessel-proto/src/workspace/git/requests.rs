//! The git calls a client makes: the reads (`git.status`, `git.branches`,
//! `git.log`, `git.stashes`, `git.show`) and the changes, with the
//! choices they take (which files, what a commit takes, what to do with
//! the stash).

use serde::{Deserialize, Serialize};

use super::WorkspaceRoot;
use crate::workspace::path::RelPath;
use crate::workspace::text::{BranchName, CommitId, CommitMessage, RemoteName};
use crate::workspace::worktree::WorktreeBase;

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
