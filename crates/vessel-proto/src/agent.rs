//! The structured-session timeline: Vessel's harness-independent view of an
//! agent's turn.
//!
//! The shapes follow ACP's `session/update` model (the transport used by
//! structured mode, see ADR 0003) and Paseo's closed timeline set, trimmed to
//! what clients need. They are Vessel's own types, not re-exports of the ACP
//! crate, so an ACP version bump never changes Vessel's wire format; the
//! mapping lives in `vessel-agent`.

use serde::{Deserialize, Serialize};

use crate::conversation::{AgentAbilities, AttachmentRef, Question, QuestionOutcome, TurnUsage};
use crate::id::{
    AgentMessageId, HarnessSessionId, PermissionId, PermissionOptionId, QuestionId, ToolCallId,
};
use crate::raw::RawJson;
use crate::timeline::{PermissionAction, PermissionPresentation, TimelineTaskId};

mod tool;
pub use tool::{ToolCall, ToolCallContent, ToolCallStatus, ToolCallUpdate, ToolKind, ToolLocation};

/// Where a fork or a rewind can start (protocol 14, ADR 0052): the end of a
/// turn, named as the agent names it.
///
/// [Decision Log]
/// - Purpose: ACP's `session/fork` forks the whole conversation; both
///   installed adapters also read an anchor (`_meta.jetbrains.air.fork`),
///   an agent message's id, and fork up to it (Claude) or after the turn
///   that holds it (Codex).
/// - Choice: only a turn's end is a point (a cut inside a turn would leave
///   a tool call without its result), and only when the turn's last agent
///   message carried an id and nothing but the turn's end followed it (a
///   thought, a plan or a tool call after it would be cut off). The point
///   carries the conversation it belongs to, so a fork from a point a
///   rewind left behind takes the conversation that still holds it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TurnPoint {
    /// The turn's last agent message.
    pub message: AgentMessageId,
    /// The agent's conversation it is in.
    pub conversation: HarnessSessionId,
}

/// One entry in a structured session's timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AgentEvent {
    /// The harness accepted the session; its id is kept for resume.
    Ready {
        /// The harness's session id.
        harness_session: HarnessSessionId,
        /// What the agent said it can do in its handshake (protocol 10);
        /// absent from older nodes.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        abilities: Option<AgentAbilities>,
    },
    /// A user turn began (echoes the prompt as the harness received it).
    TurnStarted {
        /// Caller identity for optimistic reconciliation, absent on legacy prompts.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_message_id: Option<crate::timeline::ClientMessageId>,
        /// The prompt.
        content: Vec<ContentBlock>,
    },
    /// An out-of-band native command was accepted. This never starts a model turn.
    CommandAccepted {
        /// Caller identity shared with its durable keyed receipt.
        client_message_id: crate::timeline::ClientMessageId,
        /// Exact original submitted content; no provider-built prompt is fabricated.
        content: Vec<ContentBlock>,
        /// Closed localizable outcome without raw RPC text or goal output.
        status: crate::native_commands::NativeCommandStatus,
    },
    /// A chunk of an earlier user message, replayed by the harness when a
    /// session is resumed. Prompts sent through Vessel appear as
    /// `TurnStarted` instead.
    UserMessageChunk {
        /// The chunk.
        content: ContentBlock,
    },
    /// A chunk of the agent's visible reply.
    MessageChunk {
        /// The chunk.
        content: ContentBlock,
    },
    /// A chunk of the agent's reasoning.
    ThoughtChunk {
        /// The chunk.
        content: ContentBlock,
    },
    /// A tool call was announced.
    ToolCall(ToolCall),
    /// A tool call progressed or finished.
    ToolCallUpdate(ToolCallUpdate),
    /// The agent's current plan (replaces any earlier plan).
    Plan {
        /// Plan entries in order.
        entries: Vec<PlanEntry>,
    },
    /// The harness asks a human to approve something.
    PermissionRequested(PermissionRequest),
    /// A pending permission request was settled (by a client or by the turn
    /// ending).
    PermissionResolved {
        /// Which request.
        id: PermissionId,
        /// How it was settled.
        outcome: PermissionOutcome,
    },
    /// The turn finished normally.
    TurnEnded {
        /// Why it stopped.
        reason: StopReason,
        /// The turn's tokens, when the agent reports them with its end
        /// (protocol 10).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<TurnUsage>,
        /// Where a fork or a rewind can start (protocol 14, ADR 0052):
        /// set when the turn's last agent message carried an id and nothing
        /// but the turn's end followed it. Absent otherwise, and from older
        /// nodes.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        point: Option<TurnPoint>,
    },
    /// The turn failed (the harness returned an error for the prompt).
    TurnFailed {
        /// Human-readable reason.
        message: String,
    },
    /// A non-terminal provider error; it does not settle the current turn.
    Error {
        /// Human-readable failure explanation.
        message: String,
    },
    /// Provider status with explicit severity, independent of turn completion.
    Notification(crate::timeline::TimelineNotification),
    /// Context maintenance progress, independent of turn completion.
    Compaction(crate::timeline::Compaction),
    /// Bounded opaque contribution interpreted only by its owning plugin.
    Plugin(crate::timeline::PluginTimelineItem),
    /// A line the harness wrote to stderr. Kept because login and
    /// configuration problems surface only there.
    HarnessLog {
        /// The line, without its newline.
        line: String,
    },
    /// A message added to the running turn (protocol 10): a prompt
    /// delivered `steer`, which the agent took into its turn.
    Steered {
        /// Caller identity for optimistic reconciliation, absent on legacy prompts.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_message_id: Option<crate::timeline::ClientMessageId>,
        /// The message.
        content: Vec<ContentBlock>,
    },
    /// The agent asks a person something (protocol 10), pending until a
    /// client answers.
    QuestionAsked(Question),
    /// A question was settled (protocol 10): answered, declined, or
    /// withdrawn by the turn or the session ending.
    QuestionResolved {
        /// Which question.
        id: QuestionId,
        /// How it was settled.
        outcome: QuestionOutcome,
    },
}

/// A piece of message content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain or markdown text.
    Text {
        /// The text.
        text: String,
    },
    /// A reference to a file or URL.
    ResourceLink {
        /// URI of the resource.
        uri: String,
        /// Display name.
        name: String,
    },
    /// Any other block (image, audio, embedded resource), forwarded verbatim
    /// in the transport's own format for clients that can render it.
    Opaque {
        /// The block as the transport sent it.
        raw: RawJson,
    },
    /// A file sent with a prompt, kept by the node (protocol 10). Only in
    /// what a client sends (a prompt, a steer, a queued prompt); what an
    /// agent sends stays `opaque`.
    Attachment {
        /// Which, and what it is.
        attachment: AttachmentRef,
    },
}

/// Priority of a plan entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PlanPriority {
    /// High.
    High,
    /// Medium.
    Medium,
    /// Low.
    Low,
}

/// Progress of a plan entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    /// Not started.
    Pending,
    /// Being worked on.
    InProgress,
    /// Done.
    Completed,
}

/// One step of the agent's plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PlanEntry {
    /// Stable provider task identity survives text and order changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<TimelineTaskId>,
    /// Running-form label reported by the provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_form: Option<String>,
    /// What the step is.
    pub content: String,
    /// Importance.
    pub priority: PlanPriority,
    /// Progress.
    pub status: PlanStatus,
}

/// What choosing an option means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PermissionOptionKind {
    /// Allow this once.
    AllowOnce,
    /// Allow this and remember.
    AllowAlways,
    /// Reject this once.
    RejectOnce,
    /// Reject this and remember.
    RejectAlways,
}

/// One choice offered by a permission request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PermissionOption {
    /// Provider semantics for this existing option id; never an invented action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<PermissionAction>,
    /// Harness-assigned option id, echoed back when chosen.
    pub id: PermissionOptionId,
    /// Label to show.
    pub name: String,
    /// Meaning.
    pub kind: PermissionOptionKind,
}

/// A request for human approval, pending until a client answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PermissionRequest {
    /// Scope of the proposed tool; absence is legacy attribution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_scope: Option<Box<crate::timeline::ToolCallScope>>,
    /// Typed approval semantics and plan Markdown, when reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<PermissionPresentation>,
    /// Node-assigned id clients answer with.
    pub id: PermissionId,
    /// Actual tool identity, absent for standalone plan/mode requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<ToolCallId>,
    /// Only dedicated provider proposals have a lifetime beyond their completed turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_proposal: Option<Box<crate::timeline::PlanProposalKey>>,
    /// In-flight or uncertain decisions cannot be offered again by another client.
    #[serde(
        default,
        skip_serializing_if = "crate::timeline::PermissionDeliveryState::is_pending"
    )]
    pub delivery_state: crate::timeline::PermissionDeliveryState,
    /// Choices, in the harness's order.
    pub options: Vec<PermissionOption>,
}

/// How a permission request was settled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PermissionOutcome {
    /// A client chose an option.
    Selected {
        /// The chosen option.
        option: PermissionOptionId,
    },
    /// Withdrawn: the turn was cancelled or the session ended first.
    Cancelled,
}

/// Why a turn stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The agent finished.
    EndTurn,
    /// Token limit reached.
    MaxTokens,
    /// Per-turn request limit reached.
    MaxTurnRequests,
    /// The model refused.
    Refusal,
    /// A client cancelled the turn.
    Cancelled,
}
