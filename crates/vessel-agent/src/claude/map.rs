//! Claude Code's frames as Vessel's timeline (`AgentEvent`): what a
//! session shows of a turn. Pure: the session loop feeds it the frames it
//! reads and publishes what it returns; no I/O here.
//!
//! [Decision Log]
//! - Purpose: the same timeline the ACP driver gives, from the CLI's own
//!   frames, so nothing above the seam knows which driver runs.
//! - Text and reasoning come from the stream (`stream_event` deltas,
//!   `--include-partial-messages`), as they are typed; the whole
//!   `assistant` frame that follows repeats them, so its text is shown
//!   only for a message that was never streamed (one the CLI makes
//!   itself, a command's output). Paseo does the same.
//! - A tool call is announced from the whole `assistant` frame, whose
//!   input is complete (the stream gives it in pieces), and settled by the
//!   `user` frame carrying its result. Each call is announced once.
//! - A subagent's frames (`parent_tool_use_id` set) belong to its parent
//!   call, not to the main conversation; until the slice that shows
//!   subagents they are left out, never mixed into the main reply.
//! - The turn's end is the `result` frame: an error subtype, or `success`
//!   with `is_error` (how the CLI reports a usage limit or a rejected
//!   request), is a failed turn with the agent's own words.

use std::collections::HashSet;
use std::path::PathBuf;

use serde_json::Value;
use vessel_proto::{
    AgentEvent, ContentBlock, RawJson, StopReason, TokenCount, ToolCall, ToolCallContent,
    ToolCallId, ToolCallStatus, ToolCallUpdate, ToolKind, ToolLocation, TurnUsage,
};

use super::wire::{
    Assistant, Block, Delta, RawEvent, StreamEvent, ToolResultContent, TurnResult, Usage,
    UserMessage,
};

/// The longest title a tool call is given, in characters.
const TITLE_CHARS: usize = 200;

/// What has been shown of the session so far.
#[derive(Debug, Default)]
pub(crate) struct Translator {
    /// Messages whose text came through the stream.
    streamed: HashSet<String>,
    /// Tool calls announced.
    announced: HashSet<String>,
}

impl Translator {
    /// What a `stream_event` frame shows.
    pub(crate) fn stream(&mut self, frame: &StreamEvent) -> Vec<AgentEvent> {
        if frame.parent_tool_use_id.is_some() {
            return Vec::new();
        }
        match &frame.event {
            RawEvent::MessageStart { message } => {
                let _new = self.streamed.insert(message.id.clone());
                Vec::new()
            }
            RawEvent::ContentBlockDelta {
                delta: Delta::Text { text },
                ..
            } if !text.is_empty() => vec![AgentEvent::MessageChunk {
                content: text_block(text.clone()),
            }],
            RawEvent::ContentBlockDelta {
                delta: Delta::Thinking { thinking },
                ..
            } if !thinking.is_empty() => vec![AgentEvent::ThoughtChunk {
                content: text_block(thinking.clone()),
            }],
            RawEvent::ContentBlockStart { .. }
            | RawEvent::ContentBlockDelta { .. }
            | RawEvent::ContentBlockStop { .. }
            | RawEvent::MessageStop
            | RawEvent::Unknown => Vec::new(),
        }
    }

    /// What an `assistant` frame shows.
    pub(crate) fn assistant(&mut self, frame: &Assistant) -> Vec<AgentEvent> {
        if frame.parent_tool_use_id.is_some() {
            return Vec::new();
        }
        let streamed = self.streamed.contains(&frame.message.id);
        let mut events = Vec::new();
        for block in &frame.message.content {
            match block {
                Block::Text { text } if !streamed && !text.is_empty() => {
                    events.push(AgentEvent::MessageChunk {
                        content: text_block(text.clone()),
                    });
                }
                Block::Thinking { thinking } if !streamed && !thinking.is_empty() => {
                    events.push(AgentEvent::ThoughtChunk {
                        content: text_block(thinking.clone()),
                    });
                }
                Block::ToolUse { id, name, input } if self.announced.insert(id.clone()) => {
                    events.push(AgentEvent::ToolCall(tool_call(id, name, input)));
                }
                Block::Text { .. }
                | Block::Thinking { .. }
                | Block::ToolUse { .. }
                | Block::ToolResult { .. }
                | Block::Unknown => {}
            }
        }
        events
    }

    /// What a `user` frame of the agent's shows: its tools' results.
    pub(crate) fn user(&self, frame: &UserMessage) -> Vec<AgentEvent> {
        if frame.parent_tool_use_id.is_some() {
            return Vec::new();
        }
        let ToolResultContent::Blocks(blocks) = &frame.message.content else {
            return Vec::new();
        };
        blocks
            .iter()
            .filter_map(|block| match block {
                Block::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                } if self.announced.contains(tool_use_id) => Some(AgentEvent::ToolCallUpdate(
                    settled(tool_use_id, content, *is_error),
                )),
                Block::ToolResult { .. }
                | Block::Text { .. }
                | Block::Thinking { .. }
                | Block::ToolUse { .. }
                | Block::Unknown => None,
            })
            .collect()
    }

    /// The turn's end, from its `result` frame; `cancelled` when the host
    /// interrupted it.
    pub(crate) fn end(result: &TurnResult, cancelled: bool) -> AgentEvent {
        if cancelled {
            return AgentEvent::TurnEnded {
                reason: StopReason::Cancelled,
                usage: result.usage.as_ref().map(turn_usage),
                point: None,
            };
        }
        if result.subtype != "success" {
            let message = if result.errors.is_empty() {
                format!("the turn ended with {}", result.subtype)
            } else {
                result.errors.join("; ")
            };
            return AgentEvent::TurnFailed { message };
        }
        if result.is_error {
            let message = result
                .result
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .unwrap_or("the agent's request failed")
                .to_owned();
            return AgentEvent::TurnFailed { message };
        }
        AgentEvent::TurnEnded {
            reason: stop_reason(result.stop_reason.as_deref()),
            usage: result.usage.as_ref().map(turn_usage),
            point: None,
        }
    }
}

fn text_block(text: String) -> ContentBlock {
    ContentBlock::Text { text }
}

/// The Messages API's stop reason as Vessel's.
fn stop_reason(reason: Option<&str>) -> StopReason {
    match reason {
        Some("max_tokens") => StopReason::MaxTokens,
        Some("refusal") => StopReason::Refusal,
        // `end_turn`, `stop_sequence`, `tool_use` (the last call of a turn
        // the CLI ended), and whatever a newer API adds.
        _ => StopReason::EndTurn,
    }
}

/// A turn's tokens: what the API counted for the main conversation.
fn turn_usage(usage: &Usage) -> TurnUsage {
    let input = usage
        .input_tokens
        .saturating_add(usage.cache_read_input_tokens)
        .saturating_add(usage.cache_creation_input_tokens);
    TurnUsage {
        total: TokenCount::new(input.saturating_add(usage.output_tokens)),
        input: TokenCount::new(input),
        output: TokenCount::new(usage.output_tokens),
        thought: None,
        cached_read: Some(TokenCount::new(usage.cache_read_input_tokens)),
        cached_write: Some(TokenCount::new(usage.cache_creation_input_tokens)),
    }
}

/// A call of `name` with `input`, announced.
fn tool_call(id: &str, name: &str, input: &Value) -> ToolCall {
    ToolCall {
        id: ToolCallId::new(id),
        title: title(name, input),
        kind: kind(name),
        status: ToolCallStatus::Pending,
        content: Vec::new(),
        locations: path(input)
            .map(|path| ToolLocation { path, line: None })
            .into_iter()
            .collect(),
        raw_input: RawJson::from_text(input.to_string()).ok(),

        detail: None,
        failure: None,
        agent_message: None,
    }
}

/// A call settled by its result.
fn settled(id: &str, content: &ToolResultContent, failed: bool) -> ToolCallUpdate {
    let text = match content {
        ToolResultContent::Text(text) => text.clone(),
        ToolResultContent::Blocks(blocks) => blocks
            .iter()
            .filter_map(|block| match block {
                Block::Text { text } => Some(text.as_str()),
                Block::Thinking { .. }
                | Block::ToolUse { .. }
                | Block::ToolResult { .. }
                | Block::Unknown => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    };
    ToolCallUpdate {
        id: ToolCallId::new(id),
        title: None,
        kind: None,
        status: Some(if failed {
            ToolCallStatus::Failed
        } else {
            ToolCallStatus::Completed
        }),
        content: (!text.is_empty()).then(|| {
            vec![ToolCallContent::Content {
                content: text_block(text),
            }]
        }),
        locations: None,
        raw_input: None,
        // The CLI's structured result (`tool_use_result`) differs per tool
        // and is not read yet; the text above is what a person sees.
        raw_output: None,

        detail: None,
        failure: None,
        agent_message: None,
    }
}

/// Claude Code's built-in tools by what they do; an MCP tool or one this
/// build does not know is `other`.
fn kind(name: &str) -> ToolKind {
    match name {
        "Read" | "NotebookRead" => ToolKind::Read,
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => ToolKind::Edit,
        "Bash" | "BashOutput" | "KillShell" | "KillBash" => ToolKind::Execute,
        "Glob" | "Grep" | "LS" | "WebSearch" | "ToolSearch" => ToolKind::Search,
        "WebFetch" => ToolKind::Fetch,
        "ExitPlanMode" | "EnterPlanMode" => ToolKind::SwitchMode,
        "TodoWrite" => ToolKind::Think,
        _ => ToolKind::Other,
    }
}

/// A call's title: the tool, and what it acts on when the input says.
fn title(name: &str, input: &Value) -> String {
    let detail = [
        "command",
        "file_path",
        "notebook_path",
        "pattern",
        "url",
        "query",
        "description",
    ]
    .iter()
    .find_map(|key| input.get(key).and_then(Value::as_str))
    .and_then(|text| text.lines().next())
    .map(str::trim)
    .filter(|text| !text.is_empty());
    let whole = match detail {
        Some(detail) => format!("{name}: {detail}"),
        None => name.to_owned(),
    };
    whole.chars().take(TITLE_CHARS).collect()
}

/// The file a call acts on, when its input names one.
fn path(input: &Value) -> Option<PathBuf> {
    ["file_path", "notebook_path"]
        .iter()
        .find_map(|key| input.get(key).and_then(Value::as_str))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests;
