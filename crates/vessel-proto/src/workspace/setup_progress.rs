//! Additive setup observations. Legacy combined output remains available.

use std::num::NonZeroU32;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod limits;
pub use limits::{InvalidSetupProgress, MAX_SETUP_COMMAND_OUTPUT_BYTES, MAX_SETUP_OUTPUT_BYTES};

use super::{MAX_PROJECT_COMMANDS, ProjectCommand};
use crate::{DurationMs, ProtocolVersion, SetupRevision, SetupRunId, TerminalBytes, WorktreeId};

/// Assigned only when the integrator merges the complete setup feature.
pub const WORKTREE_SETUP_OBSERVATION: Option<ProtocolVersion> = None;

/// A one-based ordinal within an approved setup list, matching Paseo events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "u32", into = "u32")]
pub struct SetupCommandIndex(NonZeroU32);

/// A command index outside the setup list bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("setup command index is outside its bounded list")]
pub struct InvalidSetupCommandIndex;

impl SetupCommandIndex {
    /// The displayed ordinal: the first approved command is 1.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

impl TryFrom<u32> for SetupCommandIndex {
    type Error = InvalidSetupCommandIndex;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        NonZeroU32::new(value)
            .filter(|ordinal| {
                usize::try_from(ordinal.get()).is_ok_and(|value| value <= MAX_PROJECT_COMMANDS)
            })
            .map(Self)
            .ok_or(InvalidSetupCommandIndex)
    }
}

impl From<SetupCommandIndex> for u32 {
    fn from(value: SetupCommandIndex) -> Self {
        value.get()
    }
}

/// A cursor names the generation as well as its revision, preventing an old
/// reader's counter from hiding a newer run or a node restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetupCursor {
    /// Node-minted invocation identity.
    pub run: SetupRunId,
    /// Revision within that invocation.
    pub revision: SetupRevision,
}

/// A terminal command result; logs and exit codes cannot imply cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SetupCommandResult {
    /// Process exit; absent code means signal or unknown exit.
    Exited {
        /// The reaped code, when known.
        code: Option<i32>,
    },
    /// The shell failed to spawn.
    SpawnFailed,
    /// Its process group was stopped at the setup deadline.
    TimedOut,
    /// This invocation was cancelled.
    Cancelled,
    /// Command preparation or log writing failed.
    IoFailed,
    /// A newer node's result.
    #[serde(other)]
    Unknown,
}

/// A command is never marked complete by reading printable output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SetupCommandState {
    /// Approved, but not reached by the runner.
    Waiting,
    /// Preparation/execution is in progress.
    Running,
    /// The runner reported a terminal result.
    Finished {
        /// Monotonic elapsed time, independent of wall-clock changes.
        elapsed: DurationMs,
        /// Exact runner result.
        result: SetupCommandResult,
    },
    /// A newer node's state.
    #[serde(other)]
    Unknown,
}

/// A per-command display value. This DTO alone does not validate output limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetupCommandProgress {
    /// Position in the exact approved invocation.
    pub index: SetupCommandIndex,
    /// Approved text; never reread from the worktree during execution.
    pub command: ProjectCommand,
    /// Current structured state.
    pub state: SetupCommandState,
    /// The producer must enforce its output budget; this generic byte value does not.
    pub output: TerminalBytes,
    /// Bytes were omitted from the middle, independently of the global log cap.
    pub truncated: bool,
}

/// Overall state of an observed invocation, separate from legacy `SetupState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SetupRunState {
    /// Commands are still being processed.
    Running,
    /// Every approved command succeeded.
    Succeeded,
    /// Execution or observation failed.
    Failed,
    /// The runner stopped following explicit cancellation.
    Cancelled,
    /// A newer node's state, never successful completion.
    #[serde(other)]
    Unknown,
}

/// One volatile runner snapshot. A restarted node does not reconstruct
/// command boundaries from a surviving combined log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetupProgress {
    /// Exact backing worktree, not a cwd-based ownership inference.
    pub worktree: WorktreeId,
    /// Invocation and revision.
    pub cursor: SetupCursor,
    /// Structured overall state.
    pub state: SetupRunState,
    /// Wall-clock start for display.
    pub started_at: DateTime<Utc>,
    /// Decoding checks count, ordered ordinals and per-command/aggregate output.
    /// Locally constructed DTOs must call `validate` before renderer admission.
    #[serde(deserialize_with = "limits::decode_commands")]
    pub commands: Vec<SetupCommandProgress>,
}

/// Read once or wait for a newer structured snapshot (bounded node-side).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadSetupProgress {
    /// Backing worktree resource.
    pub worktree: WorktreeId,
    /// Wait only while this exact cursor remains current.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<SetupCursor>,
    /// Maximum wait in milliseconds; the node applies its usual long-poll cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_ms: Option<DurationMs>,
}

/// Absence means no structured observation is available, not success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetupProgressReply {
    /// Current invocation, if observed by this node process.
    pub progress: Option<SetupProgress>,
}

/// Stop only the generation the person actually saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CancelSetup {
    /// Backing worktree resource.
    pub worktree: WorktreeId,
    /// A stale action cannot stop a newer invocation.
    pub run: SetupRunId,
}

/// Cancellation is a request until the runner reports its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CancelSetupOutcome {
    /// Token signalled; observe the terminal state before claiming completion.
    Requested,
    /// The observed generation already ended.
    NotRunning,
    /// No observation, or another generation is current; nothing signalled.
    Stale,
    /// A newer node's result.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests;
