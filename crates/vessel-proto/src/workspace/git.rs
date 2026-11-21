//! Git: status, branches, history and what changes them (protocol 10,
//! ADR 0023). Expected refusals are typed outcomes ([`GitRefusal`]), not
//! errors, so no client reads git's own text.
//!
//! This module names the folder every call works in ([`WorkspaceRoot`]);
//! what the reads answer is `state`'s, what a change answers `outcomes`',
//! and the calls themselves `requests`'. Every type is re-exported here
//! and again by `workspace`: clients name them `vessel_proto::workspace::`
//! and never by these submodules.

mod origin;
mod outcomes;
mod requests;
mod state;

use serde::{Deserialize, Serialize};

use crate::id::{ProjectId, WorktreeId};

pub use origin::{ORIGIN_LINK_PROTOCOL, OriginRepo, serves_origin};
pub use outcomes::{
    CommitOutcome, FetchOutcome, GitRefusal, PullOutcome, PushOutcome, RenameOutcome,
    RepositoryConfigReason, StashOutcome, SwitchOutcome,
};
pub use requests::{
    Commit, CommitStage, Discard, Fetch, GetStatus, ListBranches, ListStashes, PathSet, Pull, Push,
    ReadLog, RenameBranch, ShowFile, Stage, StashAction, StashChanges, Switch, Unstage,
};
pub use state::{
    Branch, BranchList, CommitInfo, CommitPage, Distance, FileState, GitStatus, StashEntry,
    StashList, StatusEntry,
};

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
