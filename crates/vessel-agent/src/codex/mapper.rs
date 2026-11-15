//! Codex's app-server messages as Vessel's events, and Vessel's prompts as
//! Codex's input: pure functions, as `crate::mapper` is for ACP.
//!
//! [Decision Log]
//! - Purpose: the same timeline whichever driver runs a session; the
//!   session actors, the store and clients never learn which one did.
//! - Items: a tool-like item (a command, a file change, an MCP tool call, a
//!   web search) is a tool call announced at `item/started` and settled at
//!   `item/completed`; Codex's item id is the tool call's id. The agent's
//!   text and its reasoning summary stream as chunks, and an item that came
//!   without deltas is shown whole when it completes.
//! - Tokens: `thread/tokenUsage/updated` carries the thread's running
//!   total and the last request's. A turn's usage is the total's growth
//!   over the turn (a turn may make several requests); the context's
//!   fullness is the last request's tokens against the model's window.
//! - A declined item (the driver declines every approval until its
//!   permissions slice) ends `failed`, as ACP shows a rejected call.

use std::fmt::Write as _;
use std::path::PathBuf;

use serde_json::Value;
use vessel_proto::conversation::UsageObservation;
use vessel_proto::{
    AgentEvent, ContentBlock, ContextUsage, PlanEntry, PlanPriority, PlanStatus, RawJson,
    StopReason, TokenCount, ToolCall, ToolCallContent, ToolCallId, ToolCallStatus, ToolCallUpdate,
    ToolKind, ToolLocation, TurnUsage,
};

use super::protocol::{
    ItemStatus, ThreadItem, ThreadTokenUsage, TokenUsageBreakdown, Turn, TurnPlanStep,
    TurnPlanStepStatus, TurnStatus, UserInput,
};
use crate::error::InputError;

/// A prompt's content as Codex's input. Text goes as text; a link to a
/// file as a mention of it (the agent reads it with its own tools), any
/// other link as its text.
///
/// # Errors
/// [`InputError::AttachmentNotTaken`] for an attachment (Codex's driver
/// takes none yet), [`InputError::InvalidContent`] for an opaque block.
pub(super) fn user_input(content: &[ContentBlock]) -> Result<Vec<UserInput>, InputError> {
    content
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => Ok(UserInput::Text { text: text.clone() }),
            ContentBlock::ResourceLink { uri, name } => {
                Ok(uri.strip_prefix("file://").and_then(decoded).map_or_else(
                    || UserInput::Text { text: uri.clone() },
                    |path| UserInput::Mention {
                        name: name.clone(),
                        path,
                    },
                ))
            }
            ContentBlock::Attachment { attachment } => Err(InputError::AttachmentNotTaken {
                id: attachment.id,
                kind: attachment.kind,
            }),
            ContentBlock::Opaque { .. } => Err(InputError::InvalidContent(
                serde::de::Error::custom("Codex takes text and links to files"),
            )),
        })
        .collect()
}

/// A `file:` URI's path with its percent escapes decoded (`%20` is a
/// space); `None` when an escape is broken or the bytes are not UTF-8, and
/// the link then goes as its text.
fn decoded(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        if byte == b'%' {
            let hex = bytes.get(at.checked_add(1)?..at.checked_add(3)?)?;
            let text = std::str::from_utf8(hex).ok()?;
            out.push(u8::from_str_radix(text, 16).ok()?);
            at = at.checked_add(3)?;
        } else {
            out.push(byte);
            at = at.checked_add(1)?;
        }
    }
    String::from_utf8(out).ok()
}

/// The tool call a started item announces, for a tool-like item.
pub(super) fn tool_call(item: &ThreadItem) -> Option<ToolCall> {
    let (id, title, kind, status, raw_input) = match item {
        ThreadItem::CommandExecution {
            id,
            command,
            cwd,
            status,
            ..
        } => {
            let input = serde_json::json!({ "command": command, "cwd": cwd });
            (id, command.clone(), ToolKind::Execute, *status, Some(input))
        }
        ThreadItem::FileChange {
            id,
            changes,
            status,
        } => {
            let paths: Vec<&str> = changes.iter().map(|change| change.path.as_str()).collect();
            let title = format!("Edit {}", paths.join(", "));
            (id, title, ToolKind::Edit, *status, None)
        }
        ThreadItem::McpToolCall {
            id,
            server,
            tool,
            arguments,
            status,
        } => {
            let title = format!("{server}: {tool}");
            (id, title, ToolKind::Other, *status, Some(arguments.clone()))
        }
        ThreadItem::WebSearch { id, query } => {
            let input = serde_json::json!({ "query": query });
            let title = format!("Search the web: {query}");
            (
                id,
                title,
                ToolKind::Fetch,
                ItemStatus::InProgress,
                Some(input),
            )
        }
        ThreadItem::AgentMessage { .. } | ThreadItem::Reasoning { .. } | ThreadItem::Other => {
            return None;
        }
    };
    Some(ToolCall {
        scope: None,
        id: ToolCallId::new(id.as_str()),
        title,
        kind,
        status: status_of(status),
        content: Vec::new(),
        locations: locations(item),
        raw_input: raw_input.as_ref().and_then(raw),

        detail: None,
        failure: None,
        agent_message: None,
    })
}

/// The update a completed item settles its tool call with, for a tool-like
/// item.
pub(super) fn tool_call_done(item: &ThreadItem) -> Option<ToolCallUpdate> {
    let (id, status, content) = match item {
        ThreadItem::CommandExecution {
            id,
            status,
            aggregated_output,
            exit_code,
            ..
        } => {
            let mut text = aggregated_output.clone().unwrap_or_default();
            if let Some(code) = exit_code.filter(|code| *code != 0) {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                let _infallible = write!(text, "exit code {code}");
            }
            (id, *status, text_content(text))
        }
        ThreadItem::FileChange {
            id,
            changes,
            status,
        } => {
            let diffs = changes
                .iter()
                .map(|change| ToolCallContent::Content {
                    content: ContentBlock::Text {
                        text: change.diff.clone(),
                    },
                })
                .collect();
            (id, *status, diffs)
        }
        ThreadItem::McpToolCall { id, status, .. } => (id, *status, Vec::new()),
        ThreadItem::WebSearch { id, .. } => (id, ItemStatus::Completed, Vec::new()),
        ThreadItem::AgentMessage { .. } | ThreadItem::Reasoning { .. } | ThreadItem::Other => {
            return None;
        }
    };
    Some(ToolCallUpdate {
        scope: None,
        id: ToolCallId::new(id.as_str()),
        title: None,
        kind: None,
        status: Some(match status {
            // Completed is what `item/completed` means, whatever the item's
            // own field says.
            ItemStatus::InProgress | ItemStatus::Completed | ItemStatus::Other => {
                ToolCallStatus::Completed
            }
            ItemStatus::Failed | ItemStatus::Declined => ToolCallStatus::Failed,
        }),
        content: (!content.is_empty()).then_some(content),
        locations: None,
        raw_input: None,
        raw_output: None,

        detail: None,
        failure: None,
        agent_message: None,
    })
}

/// The agent's text of a completed item, for one that streamed no delta.
pub(super) fn whole_text(item: &ThreadItem) -> Option<(&str, AgentEvent)> {
    match item {
        ThreadItem::AgentMessage { id, text, .. } if !text.is_empty() => Some((
            id.as_str(),
            AgentEvent::MessageChunk {
                content: ContentBlock::Text { text: text.clone() },
            },
        )),
        ThreadItem::Reasoning { id, summary } if !summary.is_empty() => Some((
            id.as_str(),
            AgentEvent::ThoughtChunk {
                content: ContentBlock::Text {
                    text: summary.join("\n\n"),
                },
            },
        )),
        ThreadItem::AgentMessage { .. }
        | ThreadItem::Reasoning { .. }
        | ThreadItem::CommandExecution { .. }
        | ThreadItem::FileChange { .. }
        | ThreadItem::McpToolCall { .. }
        | ThreadItem::WebSearch { .. }
        | ThreadItem::Other => None,
    }
}

/// `turn/plan/updated` as a plan.
pub(super) fn plan(steps: &[TurnPlanStep]) -> AgentEvent {
    AgentEvent::Plan {
        entries: steps
            .iter()
            .map(|step| PlanEntry {
                content: step.step.clone(),
                priority: PlanPriority::Medium,
                status: match step.status {
                    TurnPlanStepStatus::Completed => PlanStatus::Completed,
                    TurnPlanStepStatus::InProgress => PlanStatus::InProgress,
                    TurnPlanStepStatus::Pending | TurnPlanStepStatus::Other => PlanStatus::Pending,
                },

                id: None,
                active_form: None,
            })
            .collect(),
    }
}

/// How full the context is: the last request's tokens against the model's
/// window, when Codex reports the window.
pub(super) fn context(usage: &ThreadTokenUsage) -> Option<ContextUsage> {
    let size = usage.model_context_window.filter(|size| *size > 0)?;
    Some(ContextUsage {
        // The session runtime marks a report made while loading as
        // replayed (ADR 0042); the driver cannot tell, as over ACP.
        observation: UsageObservation::Current,
        used: TokenCount::new(usage.last.total_tokens),
        size: TokenCount::new(size),
        cost: None,
    })
}

/// A turn's tokens: what the thread's total grew by from `before` to
/// `after`.
pub(super) fn turn_usage(
    before: TokenUsageBreakdown,
    after: TokenUsageBreakdown,
) -> Option<TurnUsage> {
    let grew = |before: u64, after: u64| after.saturating_sub(before);
    let total = grew(before.total_tokens, after.total_tokens);
    (total > 0).then(|| TurnUsage {
        total: TokenCount::new(total),
        input: TokenCount::new(grew(before.input_tokens, after.input_tokens)),
        output: TokenCount::new(grew(before.output_tokens, after.output_tokens)),
        thought: Some(TokenCount::new(grew(
            before.reasoning_output_tokens,
            after.reasoning_output_tokens,
        ))),
        cached_read: Some(TokenCount::new(grew(
            before.cached_input_tokens,
            after.cached_input_tokens,
        ))),
        cached_write: None,
    })
}

/// How a completed turn ends the timeline's turn.
pub(super) fn turn_end(turn: &Turn, usage: Option<TurnUsage>) -> AgentEvent {
    match turn.status {
        TurnStatus::Failed => AgentEvent::TurnFailed {
            message: turn.error.as_ref().map_or_else(
                || "the turn failed".to_owned(),
                |error| error.message.clone(),
            ),
        },
        TurnStatus::Interrupted => AgentEvent::TurnEnded {
            reason: StopReason::Cancelled,
            usage,
            point: None,
        },
        TurnStatus::Completed | TurnStatus::InProgress | TurnStatus::Other => {
            AgentEvent::TurnEnded {
                reason: StopReason::EndTurn,
                usage,
                point: None,
            }
        }
    }
}

const fn status_of(status: ItemStatus) -> ToolCallStatus {
    match status {
        ItemStatus::InProgress | ItemStatus::Other => ToolCallStatus::InProgress,
        ItemStatus::Completed => ToolCallStatus::Completed,
        ItemStatus::Failed | ItemStatus::Declined => ToolCallStatus::Failed,
    }
}

fn locations(item: &ThreadItem) -> Vec<ToolLocation> {
    match item {
        ThreadItem::FileChange { changes, .. } => changes
            .iter()
            .map(|change| ToolLocation {
                path: PathBuf::from(&change.path),
                line: None,
            })
            .collect(),
        ThreadItem::AgentMessage { .. }
        | ThreadItem::Reasoning { .. }
        | ThreadItem::CommandExecution { .. }
        | ThreadItem::McpToolCall { .. }
        | ThreadItem::WebSearch { .. }
        | ThreadItem::Other => Vec::new(),
    }
}

fn text_content(text: String) -> Vec<ToolCallContent> {
    if text.is_empty() {
        return Vec::new();
    }
    vec![ToolCallContent::Content {
        content: ContentBlock::Text { text },
    }]
}

fn raw(value: &Value) -> Option<RawJson> {
    RawJson::from_text(value.to_string()).ok()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn item(value: Value) -> ThreadItem {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn a_prompt_is_text_and_mentions() {
        let content = [
            ContentBlock::Text {
                text: "look".to_owned(),
            },
            ContentBlock::ResourceLink {
                uri: "file:///work/a.rs".to_owned(),
                name: "a.rs".to_owned(),
            },
            ContentBlock::ResourceLink {
                uri: "https://example.org".to_owned(),
                name: "site".to_owned(),
            },
            ContentBlock::ResourceLink {
                uri: "file:///work/a%20b.rs".to_owned(),
                name: "a b.rs".to_owned(),
            },
            ContentBlock::ResourceLink {
                uri: "file:///work/broken%2".to_owned(),
                name: "broken".to_owned(),
            },
        ];
        assert_eq!(
            user_input(&content).unwrap(),
            [
                UserInput::Text {
                    text: "look".to_owned()
                },
                UserInput::Mention {
                    name: "a.rs".to_owned(),
                    path: "/work/a.rs".to_owned()
                },
                UserInput::Text {
                    text: "https://example.org".to_owned()
                },
                UserInput::Mention {
                    name: "a b.rs".to_owned(),
                    path: "/work/a b.rs".to_owned()
                },
                UserInput::Text {
                    text: "file:///work/broken%2".to_owned()
                },
            ]
        );
    }

    #[test]
    fn a_command_is_a_tool_call_from_start_to_end() {
        let started = item(json!({
            "type": "commandExecution", "id": "c1", "command": "ls", "cwd": "/w",
            "commandActions": [], "status": "inProgress"
        }));
        let call = tool_call(&started).unwrap();
        assert_eq!(call.id, ToolCallId::new("c1"));
        assert_eq!(
            (call.kind, call.status),
            (ToolKind::Execute, ToolCallStatus::InProgress)
        );
        let done = item(json!({
            "type": "commandExecution", "id": "c1", "command": "ls", "cwd": "/w",
            "commandActions": [], "status": "failed", "aggregatedOutput": "nope", "exitCode": 2
        }));
        let update = tool_call_done(&done).unwrap();
        assert_eq!(update.status, Some(ToolCallStatus::Failed));
        assert_eq!(
            update.content,
            Some(vec![ToolCallContent::Content {
                content: ContentBlock::Text {
                    text: "nope\nexit code 2".to_owned()
                }
            }])
        );
        let declined = item(json!({
            "type": "fileChange", "id": "f1", "status": "declined",
            "changes": [{"path": "/w/a", "kind": {"type": "add"}, "diff": "+x"}]
        }));
        assert_eq!(
            tool_call(&declined).map(|call| call.locations),
            Some(vec![ToolLocation {
                path: PathBuf::from("/w/a"),
                line: None
            }])
        );
        assert_eq!(
            tool_call_done(&declined).and_then(|update| update.status),
            Some(ToolCallStatus::Failed)
        );
        assert!(tool_call(&item(json!({"type": "contextCompaction", "id": "x"}))).is_none());
    }

    #[test]
    fn a_turns_usage_is_what_the_total_grew_by() {
        let at = |total, input| TokenUsageBreakdown {
            total_tokens: total,
            input_tokens: input,
            ..TokenUsageBreakdown::default()
        };
        let usage = turn_usage(at(100, 80), at(160, 120)).unwrap();
        assert_eq!(usage.total, TokenCount::new(60));
        assert_eq!(usage.input, TokenCount::new(40));
        assert!(turn_usage(at(100, 80), at(100, 80)).is_none());
    }
}
