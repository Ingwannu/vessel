//! Claude Code's stream-json frames, one JSON object a line each way (ADR
//! 0057, "Claude Code"): what the driver reads from the agent's stdout and
//! writes to its stdin. Field names follow the Agent SDK's declared types
//! (`sdk.d.ts` of the version followed, 0.3.246, which is not copied
//! here); only the fields the driver uses are named.
//!
//! [Decision Log]
//! - Purpose: one place where Claude Code's wire is spelled, so the mapper
//!   and the session loop handle Rust types, never raw JSON.
//! - Two layers (ADR 0057): the frames Anthropic documents for
//!   `--output-format stream-json` (`system`, `assistant`, `user`,
//!   `stream_event`, `result`) and the Agent SDK's control frames
//!   (`control_request`, `control_response`, `control_cancel_request`,
//!   `keep_alive`), which are its internal wire and are used only where the
//!   contract test passes.
//! - Tolerant reading: a frame type, subtype, content block or delta this
//!   build does not know decodes as `Unknown` (the CLI adds frames far more
//!   often than it removes them, and the SDK's own types say to ignore
//!   unknown ones); unknown fields are ignored. What the floor needs to be
//!   strict about (a control request the driver cannot answer) is decided
//!   by the session loop, which denies it, never here.
//! - Values the agent gives as arbitrary JSON (a tool's input, a tool
//!   result's structured output) stay `serde_json::Value`: their shape is
//!   the tool's, not the protocol's.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One line of the agent's stdout.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Frame {
    /// A system message: the session's `init`, a retry, a denial.
    System(System),
    /// A message of the agent's (whole, or one content block of it).
    Assistant(Assistant),
    /// A user message: a tool's result, or a prompt the agent replays.
    User(UserMessage),
    /// One event of the streaming Messages API (`--include-partial-messages`).
    StreamEvent(StreamEvent),
    /// The end of a turn: exactly one per turn.
    Result(TurnResult),
    /// A request of the agent's to the host (internal wire).
    ControlRequest(ControlRequest),
    /// The agent's answer to a request of the host's (internal wire).
    ControlResponse(ControlResponse),
    /// The agent withdraws a request it made (internal wire).
    ControlCancelRequest(ControlCancel),
    /// Nothing; the connection is alive.
    KeepAlive,
    /// A frame this build does not read.
    #[serde(other)]
    Unknown,
}

/// A `system` frame, by subtype.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub(crate) enum System {
    /// The session started: its id, model and what the CLI can do.
    Init(Init),
    /// An API request failed and is retried.
    ApiRetry(ApiRetry),
    /// A tool call was denied without asking (a rule, a mode).
    PermissionDenied(PermissionDenied),
    /// Any other system message.
    #[serde(other)]
    Unknown,
}

/// `system`/`init`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Init {
    /// The conversation's id (what `--resume` takes).
    pub(crate) session_id: String,
    /// The CLI's version.
    #[serde(default)]
    pub(crate) claude_code_version: Option<String>,
    /// The model the session runs.
    #[serde(default)]
    pub(crate) model: Option<String>,
    /// The permission mode it runs in.
    #[serde(default, rename = "permissionMode")]
    pub(crate) permission_mode: Option<String>,
    /// The slash commands it takes, by name.
    #[serde(default)]
    pub(crate) slash_commands: Vec<String>,
    /// The protocol behaviours this CLI implements (an open set).
    #[serde(default)]
    pub(crate) capabilities: Vec<String>,
}

/// `system`/`api_retry`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct ApiRetry {
    /// The attempt, from 1.
    pub(crate) attempt: u32,
    /// How many are allowed.
    pub(crate) max_retries: u32,
    /// The error's category.
    #[serde(default)]
    pub(crate) error: Option<String>,
}

/// `system`/`permission_denied`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct PermissionDenied {
    /// The tool.
    pub(crate) tool_name: String,
    /// The call.
    pub(crate) tool_use_id: String,
}

/// An `assistant` frame.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct Assistant {
    /// The message, or the block of it this frame delivers.
    pub(crate) message: ApiMessage,
    /// The tool call of the main conversation that started the subagent
    /// this is from; `None` for the main conversation.
    #[serde(default)]
    pub(crate) parent_tool_use_id: Option<String>,
    /// This frame's id.
    #[serde(default)]
    pub(crate) uuid: Option<String>,
    /// The request failed (a rate limit, an overload, a refusal of the
    /// account).
    #[serde(default)]
    pub(crate) error: Option<String>,
    /// The message was cut by an interrupt.
    #[serde(default)]
    pub(crate) aborted: bool,
}

/// A Messages API message as the agent reports it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct ApiMessage {
    /// The API's id for it; several frames share one.
    pub(crate) id: String,
    /// Its content blocks.
    #[serde(default)]
    pub(crate) content: Vec<Block>,
}

/// A content block.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Block {
    /// Text.
    Text {
        /// The text.
        text: String,
    },
    /// Reasoning (its text may be empty: the API omits it by default).
    Thinking {
        /// The reasoning.
        #[serde(default)]
        thinking: String,
    },
    /// A tool call.
    ToolUse {
        /// The call's id.
        id: String,
        /// The tool.
        name: String,
        /// Its input, the tool's own shape.
        #[serde(default)]
        input: Value,
    },
    /// A tool's result (in a `user` frame).
    ToolResult {
        /// The call it answers.
        tool_use_id: String,
        /// The result: text, or blocks.
        #[serde(default)]
        content: ToolResultContent,
        /// The tool failed.
        #[serde(default)]
        is_error: bool,
    },
    /// A block this build does not read (images, server tools, redacted
    /// reasoning).
    #[serde(other)]
    Unknown,
}

/// A tool result's content.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub(crate) enum ToolResultContent {
    /// Text.
    Text(String),
    /// Blocks.
    Blocks(Vec<Block>),
}

impl Default for ToolResultContent {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

/// A `user` frame (the agent's own: tool results, a replayed prompt).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct UserMessage {
    /// The message.
    pub(crate) message: UserContent,
    /// As on [`Assistant`].
    #[serde(default)]
    pub(crate) parent_tool_use_id: Option<String>,
    /// Added by the CLI, not typed by a person (a tool's result is not).
    #[serde(default, rename = "isSynthetic")]
    pub(crate) synthetic: bool,
}

/// A user message's content: text, or blocks.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct UserContent {
    /// The content.
    pub(crate) content: ToolResultContent,
}

/// A `stream_event` frame.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct StreamEvent {
    /// The streaming event.
    pub(crate) event: RawEvent,
    /// As on [`Assistant`].
    #[serde(default)]
    pub(crate) parent_tool_use_id: Option<String>,
}

/// One event of the streaming Messages API.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum RawEvent {
    /// A message begins.
    MessageStart {
        /// The message, empty yet.
        message: ApiMessage,
    },
    /// A content block begins at `index`.
    ContentBlockStart {
        /// Its place in the message.
        index: u32,
        /// The block, empty yet (a tool call's id and name).
        content_block: Block,
    },
    /// More of the block at `index`.
    ContentBlockDelta {
        /// Its place.
        index: u32,
        /// What came.
        delta: Delta,
    },
    /// The block at `index` is complete.
    ContentBlockStop {
        /// Its place.
        index: u32,
    },
    /// The message ends (`message_delta` then `message_stop`).
    MessageStop,
    /// Any other event (`message_delta`, `ping`).
    #[serde(other)]
    Unknown,
}

/// A content block's increment.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Delta {
    /// Text.
    #[serde(rename = "text_delta")]
    Text {
        /// The text.
        text: String,
    },
    /// Reasoning.
    #[serde(rename = "thinking_delta")]
    Thinking {
        /// The reasoning.
        thinking: String,
    },
    /// Part of a tool call's input, as JSON text.
    #[serde(rename = "input_json_delta")]
    InputJson {
        /// The text.
        partial_json: String,
    },
    /// Any other (a signature).
    #[serde(other)]
    Unknown,
}

/// A `result` frame: the turn's end.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct TurnResult {
    /// `success`, or how it failed (`error_during_execution`,
    /// `error_max_turns`, …).
    pub(crate) subtype: String,
    /// The turn failed (also with subtype `success`: an API error such as
    /// a usage limit is reported so, its text in `result`).
    #[serde(default)]
    pub(crate) is_error: bool,
    /// The turn's final text, on `success`.
    #[serde(default)]
    pub(crate) result: Option<String>,
    /// Why the model stopped (`end_turn`, `max_tokens`, …).
    #[serde(default)]
    pub(crate) stop_reason: Option<String>,
    /// The errors, on a failure subtype.
    #[serde(default)]
    pub(crate) errors: Vec<String>,
    /// The main loop's tokens this turn.
    #[serde(default)]
    pub(crate) usage: Option<Usage>,
    /// The conversation's id.
    #[serde(default)]
    pub(crate) session_id: Option<String>,
}

/// Tokens, as the Messages API counts them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[expect(
    clippy::struct_field_names,
    reason = "the Messages API's own names, read as they are written"
)]
pub(crate) struct Usage {
    /// Input tokens not read from the cache.
    #[serde(default)]
    pub(crate) input_tokens: u64,
    /// Output tokens.
    #[serde(default)]
    pub(crate) output_tokens: u64,
    /// Input tokens written to the cache.
    #[serde(default)]
    pub(crate) cache_creation_input_tokens: u64,
    /// Input tokens read from the cache.
    #[serde(default)]
    pub(crate) cache_read_input_tokens: u64,
}

/// A `control_request` of the agent's.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct ControlRequest {
    /// Echoed by the answer.
    pub(crate) request_id: String,
    /// What it asks.
    pub(crate) request: AgentRequest,
}

/// What the agent asks the host.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub(crate) enum AgentRequest {
    /// May a tool run? (`--permission-prompt-tool stdio`).
    CanUseTool(CanUseTool),
    /// Anything else (hooks, MCP messages, elicitations): the driver
    /// registers none of those, so it answers each with an error and, for
    /// a permission's sake, never an allow.
    #[serde(other)]
    Unknown,
}

/// `can_use_tool`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct CanUseTool {
    /// The tool.
    pub(crate) tool_name: String,
    /// Its input.
    #[serde(default)]
    pub(crate) input: Value,
    /// The call.
    pub(crate) tool_use_id: String,
    /// The agent's own choices for remembering the answer.
    #[serde(default)]
    pub(crate) permission_suggestions: Vec<Value>,
    /// A title for the prompt, when the agent gives one.
    #[serde(default)]
    pub(crate) title: Option<String>,
}

/// A `control_response` of the agent's.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct ControlResponse {
    /// The answer.
    pub(crate) response: Answer,
}

/// An answer to a request of the host's.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub(crate) enum Answer {
    /// Handled.
    Success {
        /// The request answered.
        request_id: String,
        /// Its payload, shaped by the request.
        #[serde(default)]
        response: Option<Value>,
    },
    /// Refused or failed.
    Error {
        /// The request answered.
        request_id: String,
        /// Why.
        error: String,
    },
}

/// A `control_cancel_request` of the agent's.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct ControlCancel {
    /// The request withdrawn.
    pub(crate) request_id: String,
}

/// One line the host writes to the agent's stdin.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Outgoing {
    /// A prompt.
    User {
        /// The message.
        message: OutgoingMessage,
        /// Always `null`: a prompt to the main conversation.
        parent_tool_use_id: Option<String>,
        /// The conversation, once known.
        session_id: String,
    },
    /// A request of the host's.
    ControlRequest {
        /// Chosen by the host, unique among its requests in flight.
        request_id: String,
        /// What it asks.
        request: HostRequest,
    },
    /// The host's answer to a request of the agent's.
    ControlResponse {
        /// The answer.
        response: HostAnswer,
    },
}

/// A prompt's message.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct OutgoingMessage {
    /// Always `user`.
    pub(crate) role: &'static str,
    /// Its content blocks.
    pub(crate) content: Vec<OutgoingBlock>,
}

/// A prompt's content block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum OutgoingBlock {
    /// Text.
    Text {
        /// The text.
        text: String,
    },
}

/// What the host asks the agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub(crate) enum HostRequest {
    /// Open the control protocol: the first request, before any prompt.
    /// The driver registers no hooks, MCP servers of its own or prompts
    /// here, so it sends none of the SDK's optional fields.
    Initialize {},
    /// Stop the running turn.
    Interrupt {},
}

/// The host's answer.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub(crate) enum HostAnswer {
    /// Handled, with the payload the request's kind defines.
    Success {
        /// The request answered.
        request_id: String,
        /// The payload.
        response: Value,
    },
    /// Refused.
    Error {
        /// The request answered.
        request_id: String,
        /// Why.
        error: String,
    },
}

/// A tool's permission, as `can_use_tool`'s answer carries it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "behavior", rename_all = "snake_case")]
pub(crate) enum Permission {
    /// It may run, with its input as given.
    #[expect(dead_code, reason = "the permissions slice (2) answers with it")]
    Allow {
        /// The input it runs with.
        #[serde(rename = "updatedInput")]
        updated_input: Value,
        /// The call answered.
        #[serde(rename = "toolUseID")]
        tool_use_id: String,
    },
    /// It may not run.
    Deny {
        /// Why, for the agent.
        message: String,
        /// The call answered.
        #[serde(rename = "toolUseID")]
        tool_use_id: String,
    },
}

#[cfg(test)]
mod tests;
