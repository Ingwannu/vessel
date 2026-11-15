//! Setup and child-agent launch markers, without owning their runtime behavior.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::super::{DetailIndex, TimelineName, ToolExitCode};
use super::network::ToolTruncation;
use crate::workspace::BranchName;
use crate::{DurationMs, HarnessSessionId};

/// The workspace runtime's reported status for one setup command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SetupCommandStatus {
    /// The command has started and has not settled.
    Running,
    /// It finished successfully.
    Completed,
    /// It failed; its output and exit code remain visible.
    Failed,
}

/// A setup command is a reported execution, never a new approval grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetupCommand {
    /// Original positive ordinal preserves setup order across updates.
    pub index: DetailIndex,
    /// The command text is displayed, never executed by a timeline consumer.
    pub command: String,
    /// Reported working directory.
    pub cwd: PathBuf,
    /// Output, including partial output for a still-running command.
    pub log: String,
    /// The workspace runtime's authoritative state.
    pub status: SetupCommandStatus,
    /// Absence is unknown, not a successful exit.
    pub exit_code: Option<ToolExitCode>,
    /// Reported elapsed time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<DurationMs>,
}

/// Setup belongs in the same timeline vocabulary as the tools it prepares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WorktreeSetupDetail {
    /// The workspace runtime's resulting folder.
    pub worktree_path: PathBuf,
    /// Reuse the existing validated Git branch identity.
    pub branch_name: BranchName,
    /// Combined historical output, when the producer retains it.
    pub log: String,
    /// Ordered command snapshots.
    pub commands: Vec<SetupCommand>,
    /// Reported truncation, not inferred from output length.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncated: Option<ToolTruncation>,
}

/// A child action marker preserves the provider's ordinal and readable summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SubAgentAction {
    /// The provider's positive action ordinal.
    pub index: DetailIndex,
    /// Displayed tool name; it never selects a local tool implementation.
    pub tool_name: TimelineName,
    /// Provider explanation, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// A launch marker; child navigation resolves through the sub-agent owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SubAgentDetail {
    /// Provider's child category is a name, not a local harness choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_agent_type: Option<TimelineName>,
    /// The child's task description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A provider conversation id, never assumed to be a node `SessionId`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_session_id: Option<HarnessSessionId>,
    /// Historical combined child output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
    /// Optional launch-action history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<SubAgentAction>>,
}
