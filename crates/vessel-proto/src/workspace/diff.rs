//! Diffs (protocol 10): the changed files of a scope, then one file's
//! hunks, each bounded, parsed by the node (Paseo's split: a page draws
//! typed lines, never a patch it has to parse).

use serde::{Deserialize, Serialize};

use super::git::WorkspaceRoot;
use super::path::RelPath;
use super::text::CommitId;
use super::worktree::WorktreeBase;

/// Which changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiffScope {
    /// The working tree against the index; untracked files as added.
    Unstaged,
    /// The index against `HEAD`.
    Staged,
    /// The working tree against `HEAD`; untracked files as added.
    Uncommitted,
    /// The working tree against where it forked from a base (the merge
    /// base of `HEAD` and the base): committed and uncommitted changes
    /// together; untracked files as added.
    Base {
        /// The base; a worktree's own, else the project's default branch.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base: Option<WorktreeBase>,
    },
    /// One commit against its first parent.
    Commit {
        /// Which.
        id: CommitId,
    },
}

/// How a file changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    /// New (or untracked).
    Added,
    /// Changed.
    Modified,
    /// Removed.
    Deleted,
    /// Moved, maybe changed.
    Renamed,
    /// Copied, maybe changed.
    Copied,
    /// A file became a link or the other way round.
    TypeChanged,
    /// Both sides of a merge changed it.
    Conflicted,
    /// A status a newer node knows.
    #[serde(other)]
    Unknown,
}

/// One changed file in a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FileChange {
    /// Its path.
    pub path: RelPath,
    /// Where it was, for a rename or a copy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_path: Option<RelPath>,
    /// How it changed.
    pub status: ChangeStatus,
    /// Lines added; absent for a binary file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additions: Option<u32>,
    /// Lines removed; absent for a binary file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deletions: Option<u32>,
    /// Whether git sees it as binary.
    pub binary: bool,
}

/// The changed files of a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DiffSummary {
    /// The commit compared against (the merge base, `HEAD`, a parent);
    /// absent before the first commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<CommitId>,
    /// By path, at most 5,000.
    pub files: Vec<FileChange>,
    /// Whether there were more.
    pub more: bool,
}

/// What a line of a hunk is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LineKind {
    /// On both sides.
    Context,
    /// Only on the new side.
    Added,
    /// Only on the old side.
    Removed,
    /// "No newline at end of file", for the line before.
    NoNewline,
}

/// One line of a hunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DiffLine {
    /// What it is.
    pub kind: LineKind,
    /// Its text without the end of line (not valid UTF-8 replaced), at most
    /// 4 KiB.
    pub text: String,
    /// Whether it was longer and cut.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cut: bool,
}

/// A run of changed lines and their context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Hunk {
    /// Its first line on the old side (1-based; 0 for an empty side).
    pub old_start: u32,
    /// Its lines on the old side.
    pub old_lines: u32,
    /// Its first line on the new side.
    pub new_start: u32,
    /// Its lines on the new side.
    pub new_lines: u32,
    /// The function or section git names after `@@`, when it does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Its lines.
    pub lines: Vec<DiffLine>,
}

/// What a file's diff has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiffContent {
    /// Text hunks.
    Hunks {
        /// Some or all of them (`FileDiff.more_from`).
        hunks: Vec<Hunk>,
    },
    /// A binary file: no lines.
    Binary,
    /// A submodule: which commit it pointed at, before and after.
    Submodule {
        /// Before.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<CommitId>,
        /// After.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<CommitId>,
    },
    /// No change in this scope.
    Unchanged,
    /// Its diff is larger than a call reads.
    TooLarge {
        /// How large git's output for it was, at least.
        bytes: u64,
    },
    /// A kind a newer node knows.
    #[serde(other)]
    Unknown,
}

/// A file mode change (`100644` to `100755`, say), as git prints modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ModeChange {
    /// Before, in octal digits read as a number.
    pub old: u32,
    /// After.
    pub new: u32,
}

/// One file's diff in a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FileDiff {
    /// Its path.
    pub path: RelPath,
    /// Where it was, for a rename or a copy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_path: Option<RelPath>,
    /// How it changed.
    pub status: ChangeStatus,
    /// Its mode changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<ModeChange>,
    /// What it has.
    pub content: DiffContent,
    /// The index of the first hunk not sent, when a call's 5,000 lines
    /// ran out; ask again with `from_hunk`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub more_from: Option<u32>,
}

/// `git.changes`: the changed files of a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetChanges {
    /// Where.
    pub root: WorkspaceRoot,
    /// Which changes.
    pub scope: DiffScope,
    /// Leave out changes that only touch white space.
    #[serde(default)]
    pub ignore_whitespace: bool,
}

/// `git.diff`: one file's hunks in a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetFileDiff {
    /// Where.
    pub root: WorkspaceRoot,
    /// Which changes.
    pub scope: DiffScope,
    /// Which file (its new path, for a rename).
    pub path: RelPath,
    /// Leave out changes that only touch white space.
    #[serde(default)]
    pub ignore_whitespace: bool,
    /// From this hunk on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_hunk: Option<u32>,
}
