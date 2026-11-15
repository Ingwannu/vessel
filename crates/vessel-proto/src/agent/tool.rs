//! Tool calls, statuses and output bodies shared across every harness driver.
//! This module owns display contracts and never executes the described work.

use super::ContentBlock;
use crate::id::ToolCallId;
use crate::raw::RawJson;
use crate::timeline::{AgentMessageProvenance, ToolCallDetail, ToolCallScope, ToolFailure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// What kind of work a tool call does. Lets clients pick an icon and a
/// default rendering without knowing harness tool names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// Reads files or data.
    Read,
    /// Modifies files.
    Edit,
    /// Deletes files.
    Delete,
    /// Moves or renames files.
    Move,
    /// Searches.
    Search,
    /// Runs a command.
    Execute,
    /// Internal reasoning step.
    Think,
    /// Fetches from the network.
    Fetch,
    /// Changes the session mode.
    SwitchMode,
    /// Anything else.
    Other,
}

/// Progress of a tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    /// Announced, not started (possibly awaiting permission).
    Pending,
    /// Running.
    InProgress,
    /// Finished successfully.
    Completed,
    /// Finished with an error, or was rejected.
    Failed,
    /// Canceled before successful completion; distinct from a tool failure.
    Canceled,
}

/// A file location a tool call touches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ToolLocation {
    /// Path on the node.
    pub path: PathBuf,
    /// 1-based line, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// Output attached to a tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolCallContent {
    /// Regular content.
    Content {
        /// The content.
        content: ContentBlock,
    },
    /// A file edit.
    Diff {
        /// File being changed.
        path: PathBuf,
        /// Previous text (`None` for a new file).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        old_text: Option<String>,
        /// New text.
        new_text: String,
    },
}

/// A tool call as first announced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ToolCall {
    /// Identity scope; combine with id before applying any update.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Box<ToolCallScope>>,
    /// Harness-assigned id.
    pub id: ToolCallId,
    /// Short human-readable description.
    pub title: String,
    /// Category.
    pub kind: ToolKind,
    /// Initial status.
    pub status: ToolCallStatus,
    /// Content produced so far.
    #[serde(default)]
    pub content: Vec<ToolCallContent>,
    /// Canonical provider-decoded body; absence retains the legacy raw display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<Box<ToolCallDetail>>,
    /// Provider failure explanation, independent of generic output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<ToolFailure>,
    /// Semantic source attribution, never caller authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_message: Option<Box<AgentMessageProvenance>>,
    /// Files involved.
    #[serde(default)]
    pub locations: Vec<ToolLocation>,
    /// The tool's raw input arguments, forwarded and never interpreted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_input: Option<RawJson>,
}

/// A change to an announced tool call. `None` fields are unchanged; `Some`
/// collections replace the previous value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ToolCallUpdate {
    /// Identity scope; combine with id before applying any update.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Box<ToolCallScope>>,
    /// Which call.
    pub id: ToolCallId,
    /// New title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// New category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ToolKind>,
    /// New status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolCallStatus>,
    /// Replacement content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ToolCallContent>>,
    /// Replacement canonical body; None and Unknown cannot erase a known body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<Box<ToolCallDetail>>,
    /// A reported failure, when this update includes one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<ToolFailure>,
    /// Semantic source attribution supplied with this update.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_message: Option<Box<AgentMessageProvenance>>,
    /// Replacement locations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<ToolLocation>>,
    /// The tool's raw input arguments, when the harness sends them only with
    /// an update (Claude's adapter often does). Forwarded, never interpreted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_input: Option<RawJson>,
    /// The tool's raw output, forwarded and never interpreted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_output: Option<RawJson>,
}
