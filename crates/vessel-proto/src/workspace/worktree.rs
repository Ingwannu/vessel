//! Worktrees: a folder and a branch of a project's repository per session
//! (protocol 10, ADR 0027), their setup, tidying and snapshots, and where
//! a session runs.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::path::RelPath;
use super::project::ProjectCommands;
use super::text::{BranchName, CommitId, RefName};
use crate::id::{ProjectId, SessionId, WorktreeId};
use crate::profile::Creator;
use crate::terminal::TerminalBytes;

/// Where a new branch starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorktreeBase {
    /// A branch, a remote-tracking branch or a tag.
    Ref {
        /// Its name.
        name: RefName,
    },
    /// A commit (a snapshot's, to restore it).
    Commit {
        /// Its id.
        id: CommitId,
    },
}

/// Which branch a new worktree has checked out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorktreeBranch {
    /// A new branch.
    New {
        /// Its name; `vessel/<8 hex>` when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<BranchName>,
        /// Where it starts; the project's default branch when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base: Option<WorktreeBase>,
    },
    /// A branch that exists and is checked out nowhere else.
    Existing {
        /// Its name.
        name: BranchName,
    },
    /// A new branch at a snapshot's `head`, with the snapshot's files put
    /// back as uncommitted changes.
    Snapshot {
        /// The worktree the snapshot was taken of.
        worktree: WorktreeId,
        /// The new branch's name; `vessel/<8 hex>` when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<BranchName>,
    },
}

/// Where a worktree started.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WorktreeStart {
    /// The commit.
    pub commit: CommitId,
    /// The ref it was asked by, when it was asked by one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<RefName>,
}

/// Whether a new worktree runs the project's setup commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SetupChoice {
    /// Run them (they must be approved).
    #[default]
    Run,
    /// Do not; `worktree.setup` can run them later.
    Skip,
}

/// A worktree the node made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WorktreeInfo {
    /// Its id.
    pub id: WorktreeId,
    /// Its project.
    pub project: ProjectId,
    /// Its folder on the node.
    pub path: PathBuf,
    /// The branch checked out now; absent when detached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchName>,
    /// The branch the node created for it, when it did: the only branch
    /// ever deleted with it, and the one tidying checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_branch: Option<BranchName>,
    /// Where it started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<WorktreeStart>,
    /// When it was made.
    pub created_at: DateTime<Utc>,
    /// Who asked for it (ADR 0013).
    #[serde(default)]
    pub created_by: Creator,
    /// The session it was made for, when one was (the first session
    /// started in it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_for: Option<SessionId>,
    /// The live sessions whose folder is in it.
    pub sessions: Vec<SessionId>,
    /// Its setup.
    pub setup: SetupState,
    /// Why tidying left it when its last session ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept: Option<KeptReason>,
}

/// A worktree's setup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SetupState {
    /// The project's file names no setup command.
    None,
    /// Not run: skipped when it was made.
    Skipped,
    /// Running.
    Running {
        /// Since when.
        started_at: DateTime<Utc>,
    },
    /// Every command exited with 0.
    Succeeded {
        /// When.
        finished_at: DateTime<Utc>,
    },
    /// A command failed; the ones after it did not run.
    Failed {
        /// How.
        failure: SetupFailure,
        /// When.
        finished_at: DateTime<Utc>,
    },
    /// A state a newer node knows.
    #[serde(other)]
    Unknown,
}

/// How setup failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SetupFailure {
    /// A command exited with an error.
    Exit {
        /// Its index in the setup list.
        command: u32,
        /// Its exit code; absent when a signal ended it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<i32>,
    },
    /// Setup ran past its time limit and was stopped.
    TimedOut,
    /// The shell could not be started.
    Spawn,
    /// A failure a newer node knows.
    #[serde(other)]
    Unknown,
}

/// Why tidying kept a worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum KeptReason {
    /// Changed, staged or untracked files (ignored ones not counted).
    Uncommitted {
        /// How many.
        files: u32,
    },
    /// Commits on its branch that no other branch, remote-tracking branch
    /// or tag has, on a branch the node would delete.
    UniqueCommits {
        /// How many.
        commits: u32,
    },
    /// A git operation is in progress.
    Operation {
        /// Which.
        operation: GitOperation,
    },
    /// It has submodules, whose own changes and commits are not checked.
    Submodules,
    /// Entries marked assume-unchanged or skip-worktree, whose changes
    /// `git status` does not show.
    FlaggedFiles,
    /// Refs of its own (`refs/worktree/*`, `refs/bisect/*`), which go with
    /// it.
    WorktreeRefs,
    /// A live session is in it, or its setup or teardown runs.
    InUse,
    /// It could not be inspected (git failed); kept to be safe.
    InspectionFailed,
    /// A reason a newer node knows.
    #[serde(other)]
    Unknown,
}

/// Why `worktree.create` made nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorktreeRefusal {
    /// The project is not a git repository.
    NotGit,
    /// The project is being cloned, failed to clone, or its folder is gone.
    NotReady,
    /// The repository has no commit yet.
    NoCommit,
    /// The new branch exists.
    BranchExists,
    /// The branch is checked out in another folder.
    CheckedOutElsewhere {
        /// That folder.
        path: PathBuf,
    },
    /// The base does not resolve to a commit.
    NoSuchBase,
    /// The existing branch does not exist.
    NoSuchBranch,
    /// No snapshot of that worktree.
    NoSuchSnapshot,
    /// The node, or the agent asking, has as many worktrees as allowed.
    Limit,
    /// A refusal a newer node knows.
    #[serde(other)]
    Unknown,
}

/// `worktree.create`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum CreateWorktreeOutcome {
    /// Made; its setup runs in the background.
    Created {
        /// It.
        worktree: WorktreeInfo,
    },
    /// The base's setup commands are not approved; nothing was made.
    NeedsApproval {
        /// What a person would approve.
        commands: ProjectCommands,
    },
    /// Nothing was made.
    Refused {
        /// Why.
        reason: WorktreeRefusal,
    },
}

/// What `worktree.remove` does when something would be lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum IfLost {
    /// Keep the worktree.
    #[default]
    Keep,
    /// Keep a snapshot first, then remove it (kept when the snapshot is
    /// refused).
    Snapshot,
    /// Remove it anyway, its branch too.
    Discard,
}

/// `worktree.remove`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RemoveWorktreeOutcome {
    /// Gone.
    Removed {
        /// The snapshot kept first, when one was.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        snapshot: Option<Snapshot>,
    },
    /// Nothing was removed.
    Kept {
        /// Every reason that applies.
        reasons: Vec<KeptReason>,
    },
}

/// The worktrees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WorktreeList {
    /// Oldest first, at most 1,000.
    pub worktrees: Vec<WorktreeInfo>,
    /// Whether there were more.
    pub more: bool,
}

/// A project's snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SnapshotList {
    /// Newest first, at most 1,000.
    pub snapshots: Vec<Snapshot>,
    /// Whether there were more.
    pub more: bool,
}

/// A git operation that leaves a repository between two states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum GitOperation {
    /// A merge with conflicts to resolve.
    Merge,
    /// A rebase.
    Rebase,
    /// A cherry-pick.
    CherryPick,
    /// A revert.
    Revert,
    /// A bisect.
    Bisect,
}

/// A page of a worktree's setup and teardown output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetupOutput {
    /// Where this page starts in the output.
    pub from: u64,
    /// Standard output and error together, as printed.
    pub bytes: TerminalBytes,
    /// Where the next page starts.
    pub next: u64,
    /// Whether the log reached its limit and stopped keeping output.
    pub truncated: bool,
    /// The setup's state now.
    pub setup: SetupState,
}

/// A worktree's working state kept as a commit before it was removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Snapshot {
    /// The worktree it was.
    pub worktree: WorktreeId,
    /// The branch it had checked out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchName>,
    /// The snapshot commit: the working tree, its parent `head`.
    pub commit: CommitId,
    /// What `HEAD` was.
    pub head: CommitId,
    /// When it was made.
    pub created_at: DateTime<Utc>,
}

/// Where a session runs (`SessionSpec.place`, protocol 10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionPlace {
    /// The project's folder as it is.
    Project {
        /// Which project.
        project: ProjectId,
        /// A folder inside it; the project's root when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        dir: Option<RelPath>,
    },
    /// A worktree that exists.
    Worktree {
        /// Which.
        worktree: WorktreeId,
    },
}

/// Where a session runs, as the node resolved it (`SessionInfo.workspace`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SessionWorkspace {
    /// Its project.
    pub project: ProjectId,
    /// Its worktree, when it runs in one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<WorktreeId>,
    /// The worktree's branch when the session started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchName>,
}

/// `worktree.list`: every worktree, or one project's.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListWorktrees {
    /// Only this project's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectId>,
}

/// `worktree.create`: a worktree with no session yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateWorktree {
    /// Of which project.
    pub project: ProjectId,
    /// On which branch.
    pub branch: WorktreeBranch,
    /// Whether setup runs.
    #[serde(default)]
    pub setup: SetupChoice,
}

/// `worktree.output`: a page of setup and teardown output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadSetupOutput {
    /// Which worktree.
    pub worktree: WorktreeId,
    /// From this byte of the output.
    #[serde(default)]
    pub from: u64,
    /// Wait this many seconds (at most 30) for more output or setup's end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait: Option<u32>,
}

/// `worktree.setup`: run setup now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RunSetup {
    /// Which worktree.
    pub worktree: WorktreeId,
}

/// `worktree.remove`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RemoveWorktree {
    /// Which worktree.
    pub worktree: WorktreeId,
    /// What to do when something would be lost.
    #[serde(default)]
    pub if_lost: IfLost,
    /// End the live terminals in it first (ADR 0121, Paseo's
    /// `killTerminalsForWorkspace`): the client asked the person, showing
    /// how many. Without it a live terminal refuses the removal, as before.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub end_terminals: bool,
}

/// `worktree.snapshots`: a project's snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListSnapshots {
    /// Which project.
    pub project: ProjectId,
}

/// `worktree.drop_snapshot`: delete a snapshot's ref.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DropSnapshot {
    /// Which project.
    pub project: ProjectId,
    /// The worktree the snapshot was taken of.
    pub worktree: WorktreeId,
}
