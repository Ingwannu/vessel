//! Pure conversions between ACP messages and Vessel's agent timeline.
//!
//! No I/O and no state: every function takes an ACP value and returns a
//! Vessel value (or the reverse for prompts and permission answers), so the
//! whole mapping is unit-tested against fixtures in `tests/fixtures/acp/`.
//! This is the only module that reads ACP payload fields.
//!
//! ACP enums are `#[non_exhaustive]`. Every match lists all known variants
//! and handles future ones explicitly: unknown updates are skipped (and
//! logged by the caller), unknown content is forwarded as
//! [`ContentBlock::Opaque`], and unknown kinds fall back to the most
//! cautious known value.

use agent_client_protocol::v1::schema as acp;
use serde::Serialize;

mod notices;
mod prompt;
mod question;
mod report;
mod tool;

pub(crate) use prompt::prompt_block;
pub(crate) use question::{Refusal, elicitation_response, question};
pub(crate) use report::{cut, turn_usage};
pub(crate) use tool::{
    permission_options, permission_outcome, permission_tool_call, tool_call, tool_call_update,
};
use vessel_proto::{
    AgentCommand, AgentEvent, ContentBlock, ContextUsage, PlanEntry, PlanPriority, PlanStatus,
    RawJson, StopReason,
};

/// What a `session/update` becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Update {
    /// A timeline event.
    Event(AgentEvent),
    /// The agent's options changed (protocol 9): its config options, as
    /// the full set.
    ConfigOptions(Vec<acp::SessionConfigOption>),
    /// The agent's mode changed (ACP's older modes, protocol 9).
    Mode(acp::SessionModeId),
    /// The commands the agent offers now (protocol 10), the whole list.
    Commands(Vec<AgentCommand>),
    /// How full the agent's context is (protocol 10).
    Usage(ContextUsage),
    /// Not part of Vessel's timeline (yet); logged and dropped.
    Skipped(Skipped),
}

/// Kinds of `session/update` Vessel does not put on the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Skipped {
    /// Session metadata (title) changed.
    SessionInfo,
    /// Context compaction progress.
    Compaction,
    /// An update kind newer than the ACP version Vessel pins.
    Unknown,
}

/// Map one `session/update`.
pub(crate) fn session_update(update: acp::SessionUpdate) -> Update {
    use acp::SessionUpdate as U;
    let event = match update {
        U::UserMessageChunk(chunk) => AgentEvent::UserMessageChunk {
            provenance: None,
            content: content(chunk.content),
        },
        U::AgentMessageChunk(chunk) => AgentEvent::MessageChunk {
            content: content(chunk.content),
        },
        U::AgentThoughtChunk(chunk) => AgentEvent::ThoughtChunk {
            content: content(chunk.content),
        },
        U::ToolCall(call) => AgentEvent::ToolCall(tool_call(call)),
        U::ToolCallUpdate(update) => AgentEvent::ToolCallUpdate(tool_call_update(update)),
        U::Plan(plan) => AgentEvent::Plan {
            entries: plan.entries.into_iter().map(plan_entry).collect(),
        },
        U::Notice(notice) => notices::notice(notice),
        U::AvailableCommandsUpdate(update) => return Update::Commands(report::commands(update)),
        U::CurrentModeUpdate(update) => return Update::Mode(update.current_mode_id),
        U::ConfigOptionUpdate(update) => return Update::ConfigOptions(update.config_options),
        U::SessionInfoUpdate(_) => return Update::Skipped(Skipped::SessionInfo),
        U::UsageUpdate(update) => return Update::Usage(report::usage(update)),
        U::CompactionUpdate(update) => notices::compaction(update),
        U::CompactionSummaryChunk(_) => return Update::Skipped(Skipped::Compaction),
        _ => return Update::Skipped(Skipped::Unknown),
    };
    Update::Event(event)
}

/// Map a content block. Text and resource links have Vessel equivalents;
/// everything else (images, audio, embedded resources, future kinds) is
/// forwarded verbatim for clients that can render it.
pub(crate) fn content(block: acp::ContentBlock) -> ContentBlock {
    match block {
        acp::ContentBlock::Text(text) => ContentBlock::Text { text: text.text },
        acp::ContentBlock::ResourceLink(link) => ContentBlock::ResourceLink {
            uri: link.uri,
            name: link.name,
        },
        other @ (acp::ContentBlock::Image(_)
        | acp::ContentBlock::Audio(_)
        | acp::ContentBlock::Resource(_)
        | _) => opaque(&other),
    }
}

/// Map why a turn stopped. A future reason is reported as a normal end:
/// the turn did end, and nothing more specific is known.
pub(crate) fn stop_reason(reason: acp::StopReason) -> StopReason {
    match reason {
        acp::StopReason::MaxTokens => StopReason::MaxTokens,
        acp::StopReason::MaxTurnRequests => StopReason::MaxTurnRequests,
        acp::StopReason::Refusal => StopReason::Refusal,
        acp::StopReason::Cancelled => StopReason::Cancelled,
        acp::StopReason::EndTurn | _ => StopReason::EndTurn,
    }
}

fn plan_entry(entry: acp::PlanEntry) -> PlanEntry {
    PlanEntry {
        content: entry.content,
        priority: match entry.priority {
            acp::PlanEntryPriority::High => PlanPriority::High,
            acp::PlanEntryPriority::Low => PlanPriority::Low,
            acp::PlanEntryPriority::Medium | _ => PlanPriority::Medium,
        },
        status: match entry.status {
            acp::PlanEntryStatus::InProgress => PlanStatus::InProgress,
            acp::PlanEntryStatus::Completed => PlanStatus::Completed,
            acp::PlanEntryStatus::Pending | _ => PlanStatus::Pending,
        },

        id: None,
        active_form: None,
    }
}

/// Forward an ACP value Vessel does not model, as ACP JSON.
fn opaque<T: Serialize>(value: &T) -> ContentBlock {
    match RawJson::from_serialize(value) {
        Ok(raw) => ContentBlock::Opaque { raw },
        // Derived `Serialize` on ACP types cannot fail for JSON; keep the
        // timeline intact if it ever does.
        Err(error) => {
            tracing::warn!(%error, "cannot forward ACP content");
            ContentBlock::Text {
                text: String::from("[content Vessel could not forward]"),
            }
        }
    }
}

/// An opaque ACP JSON value as [`RawJson`]. The `serde_json::Value` exists
/// only here, at the edge, and is never inspected.
fn raw_json(value: &serde_json::Value) -> Option<RawJson> {
    RawJson::from_serialize(value)
        .inspect_err(|error| tracing::warn!(%error, "cannot forward raw tool data"))
        .ok()
}

#[cfg(test)]
mod tests;
