//! The closed vocabulary of canonical tool bodies, independent of provider names.

mod execution;
mod files;
mod network;

#[cfg(test)]
mod tests;

pub use execution::{
    SetupCommand, SetupCommandStatus, SubAgentAction, SubAgentDetail, WorktreeSetupDetail,
};
pub use files::{EditDetail, ReadDetail, WriteDetail};
pub use network::{
    FetchDetail, FetchStatusCode, SearchDetail, SearchMode, SearchSource, ToolTruncation,
    WebSearchResult,
};

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{TimelineName, ToolExitCode};
use crate::RawJson;

/// One tool body's meaning; clients exhaustively render this vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolCallDetail {
    /// A command and its terminal output.
    Shell(ShellDetail),
    /// Reading a file, with its reported range.
    Read(ReadDetail),
    /// Editing a file, preserving before/after and unified formats.
    Edit(EditDetail),
    /// A complete file write, distinct from a replacement edit.
    Write(WriteDetail),
    /// Searching files or the web.
    Search(SearchDetail),
    /// Fetching a resource.
    Fetch(FetchDetail),
    /// Worktree setup progress from the workspace owner.
    WorktreeSetup(WorktreeSetupDetail),
    /// A launch marker whose child activity belongs in the sub-agent panel.
    SubAgent(SubAgentDetail),
    /// Readable tool output without provider-specific parsing in clients.
    PlainText(PlainTextDetail),
    /// Proposed plan Markdown, independent of approval state.
    Plan(PlanDetail),
    /// Arguments or output whose semantics this build does not interpret.
    Unknown(UnknownToolDetail),
}

/// Command text is content; a tool detail never authorizes executing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ShellDetail {
    /// Displayed command, preserving quoting and line breaks.
    pub command: String,
    /// Reported working directory, never a client-side execution target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    /// Output received so far, independent of success or failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// An observed exit status; absence is not success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<ToolExitCode>,
}

/// Shared icon vocabulary prevents renderers branching on arbitrary labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ToolIcon {
    /// Generic tool.
    Wrench,
    /// Command execution.
    SquareTerminal,
    /// Inspecting data.
    Eye,
    /// Editing content.
    Pencil,
    /// Finding data.
    Search,
    /// Another agent.
    Bot,
    /// Mode or generation action.
    Sparkles,
    /// Reasoning or a question.
    Brain,
    /// Spoken input or output.
    MicVocal,
}

/// Generic readable content still has a label and semantic icon when reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PlainTextDetail {
    /// Provider-supplied display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<TimelineName>,
    /// Text is content, not an executable or a new message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Absent icons use the consumer's ordinary tool icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<ToolIcon>,
}

/// A plan body must not imply that a person approved implementing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PlanDetail {
    /// The provider's complete Markdown.
    pub text: String,
}

/// The explicit opaque fallback; decoding into a tree belongs to a provider edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnknownToolDetail {
    /// Uninterpreted arguments, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<RawJson>,
    /// Uninterpreted result, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<RawJson>,
}
