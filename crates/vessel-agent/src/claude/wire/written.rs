//! What the driver writes to Claude Code's stdin: prompts, its own
//! control requests and its answers to the agent's (the other direction
//! of `wire`, which re-exports these, so callers name one module).
//!
//! [Decision Log]
//! - Purpose: `wire` holds one direction each per file, so each stays one
//!   responsibility and under the file budget; the read side is the larger
//!   and changes with every CLI release, the written side only when the
//!   driver sends something new.
//! - Exhaustive by construction: only what the driver sends is declared,
//!   and each type serializes exactly the fields the Agent SDK reads.

use serde::Serialize;
use serde_json::Value;

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
    /// It may run.
    Allow {
        /// The input it runs with: the agent's own, or a question's with
        /// its answers added.
        #[serde(rename = "updatedInput")]
        updated_input: Value,
        /// The rules or mode change the person chose with it, each one of
        /// the agent's own suggestions as it gave them (or the mode a plan
        /// approval switches to); none for an answer of this once.
        #[serde(rename = "updatedPermissions", skip_serializing_if = "Vec::is_empty")]
        updated_permissions: Vec<Value>,
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
