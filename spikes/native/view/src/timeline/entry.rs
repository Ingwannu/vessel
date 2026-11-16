//! What a structured session's timeline is made of: its entries, each
//! with a key that stays while the entry grows (the page's
//! `data/timeline-model.ts`).

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use vessel_proto::parking::ParkReason;
use vessel_proto::{
    AttachmentRef, ContentBlock, EventSeq, HarnessSessionId, ModelCall, PermissionId,
    PermissionOutcome, PermissionRequest, PlanEntry, PlanStatus, ProfileName, Question, QuestionId,
    QuestionOutcome, RawJson, SessionId, SessionStatus, StopReason, ToolCall, ToolCallContent,
    ToolCallId, ToolCallStatus, ToolCallUpdate, ToolKind, ToolLocation, TurnUsage,
};

use super::plan::PlanActivity;

/// What names an entry for as long as it lives: the record that made it,
/// or, for an entry later records change in place, the protocol's id for
/// it. The page writes these as `r12`, `tcall-1`, `p<uuid>`, `q<uuid>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntryKey {
    /// The record that made it.
    Record(EventSeq),
    /// A tool call, by the harness's id.
    Tool(ToolCallId),
    /// A permission request, by the node's id.
    Permission(PermissionId),
    /// A question, by the node's id.
    Question(QuestionId),
    /// A row of a plan update: one record can say several things changed
    /// (a step done, the next started), each its own row.
    Plan {
        /// The record.
        seq: EventSeq,
        /// Which of its rows, from 0.
        step: u32,
    },
}

/// One entry of the timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its key.
    pub key: EntryKey,
    /// The record that made it.
    pub seq: EventSeq,
    /// Exclusive last contributing record bound; gaps are not membership.
    pub through: EventSeq,
    /// Exact later records contributing to this row.
    pub extra_sources: super::source::ExtraSources,
    /// When that record was written.
    pub at: DateTime<Utc>,
    /// What it is.
    pub kind: EntryKind,
    /// How many times a later record changed it in place, so what is kept
    /// per entry (its measured height, its folded text) knows when to be
    /// worked out again. The page gets the same from a new object per
    /// change; here entries change in place.
    pub rev: u32,
}

/// A prompt the person sent, or one the harness replayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    /// Its words and links.
    pub text: String,
    /// Replayed by the harness on resume, not sent through Vessel.
    pub replayed: bool,
    /// Which turn it started, from 1 (a replayed one: the turns before it).
    pub turn: u32,
    /// The files it carried (protocol 10).
    pub attachments: Vec<AttachmentRef>,
}

/// A tool call as it stands after its updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolView {
    /// The harness's id.
    pub id: ToolCallId,
    /// Its title; empty when no record gave one (the screen then says
    /// "Tool call").
    pub title: String,
    /// What it does.
    pub kind: ToolKind,
    /// Where it stands.
    pub status: ToolCallStatus,
    /// What it produced: text, diffs.
    pub content: Vec<ToolCallContent>,
    /// The files it touches.
    pub locations: Vec<ToolLocation>,
    /// Its input, as the agent sent it.
    pub input: Option<RawJson>,
    /// Its output, as the agent sent it.
    pub output: Option<RawJson>,
}

impl ToolView {
    /// The call as the agent announced it.
    #[must_use]
    pub fn announced(call: &ToolCall) -> Self {
        Self {
            id: call.id.clone(),
            title: call.title.clone(),
            kind: call.kind,
            status: call.status,
            content: call.content.clone(),
            locations: call.locations.clone(),
            input: call.raw_input.clone(),
            output: None,
        }
    }

    /// The call `existing` (absent when the update came first) with
    /// `update` applied: each field the update names replaces the old one.
    #[must_use]
    pub fn after(existing: Option<&Self>, update: &ToolCallUpdate) -> Self {
        Self {
            id: update.id.clone(),
            title: update
                .title
                .clone()
                .or_else(|| existing.map(|tool| tool.title.clone()))
                .unwrap_or_default(),
            kind: update
                .kind
                .or_else(|| existing.map(|tool| tool.kind))
                .unwrap_or(ToolKind::Other),
            status: update
                .status
                .or_else(|| existing.map(|tool| tool.status))
                .unwrap_or(ToolCallStatus::Pending),
            content: update
                .content
                .clone()
                .or_else(|| existing.map(|tool| tool.content.clone()))
                .unwrap_or_default(),
            locations: update
                .locations
                .clone()
                .or_else(|| existing.map(|tool| tool.locations.clone()))
                .unwrap_or_default(),
            input: update
                .raw_input
                .clone()
                .or_else(|| existing.and_then(|tool| tool.input.clone())),
            output: update
                .raw_output
                .clone()
                .or_else(|| existing.and_then(|tool| tool.output.clone())),
        }
    }

    /// Whether it is still working (its card is open while it is).
    #[must_use]
    pub const fn working(&self) -> bool {
        match self.status {
            ToolCallStatus::Pending | ToolCallStatus::InProgress => true,
            ToolCallStatus::Completed | ToolCallStatus::Failed => false,
        }
    }

    /// The first file it touches, for the card's path.
    #[must_use]
    pub fn path(&self) -> Option<&PathBuf> {
        self.locations.first().map(|location| &location.path)
    }
}

/// A permission request, and how it was settled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    /// The request.
    pub request: PermissionRequest,
    /// What it is for: its tool call's title, when the call was seen.
    pub title: Option<String>,
    /// How it was settled; `None` while it waits.
    pub outcome: Option<PermissionOutcome>,
}

/// What an entry is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    /// The agent took the session (its meta line).
    Ready {
        /// The harness's own session id.
        harness_session: HarnessSessionId,
    },
    /// A prompt.
    Prompt(Prompt),
    /// A message added to the running turn (protocol 10).
    Steer {
        /// Its words.
        text: String,
        /// Its files.
        attachments: Vec<AttachmentRef>,
    },
    /// A question the agent asked; the form itself docks over the
    /// composer.
    Question {
        /// The question.
        question: Question,
        /// How it was settled; `None` while it waits.
        outcome: Option<QuestionOutcome>,
    },
    /// The agent's reply, Markdown.
    Message {
        /// Its text.
        text: String,
    },
    /// The agent's reasoning.
    Thought {
        /// Its text.
        text: String,
    },
    /// A tool call.
    Tool(ToolView),
    /// The agent's plan at that point, and what changed in it (Paseo's
    /// task activity, `plan.rs`).
    Plan {
        /// Its steps.
        entries: Vec<PlanEntry>,
        /// What changed.
        activity: PlanActivity,
    },
    /// A permission request.
    Permission(Permission),
    /// The turn ended.
    TurnEnded {
        /// Why.
        reason: StopReason,
        /// Its tokens, where the agent reported them.
        usage: Option<TurnUsage>,
        /// The agent named this turn's end (protocol 14's `point`): the
        /// conversation can be rewound or forked here.
        point: bool,
    },
    /// The turn failed.
    TurnFailed {
        /// The agent's message.
        message: String,
    },
    /// A line the harness wrote to its error output.
    Log {
        /// The line.
        line: String,
    },
    /// The session's status changed.
    Status {
        /// To what.
        status: SessionStatus,
    },
    /// The session's agent was released or resumed while the session
    /// stayed live (protocol 12, ADR 0038).
    Parking(ParkingChange),
    /// The session began as a fork of another (protocol 14, ADR 0052):
    /// its first line says where it came from (`fork.rs`).
    Forked {
        /// The session forked.
        from: SessionId,
        /// The turn's end of `from` it continues from; `None`: the whole
        /// conversation as it then stood.
        after: Option<EventSeq>,
    },
    /// The session's profile changed (protocol 5).
    Profile {
        /// From; `None` for the default.
        from: Option<ProfileName>,
        /// To; `None` for the default.
        to: Option<ProfileName>,
    },
    /// A model call through the node's proxy (protocol 7): metadata only.
    Model(ModelCall),
}

/// The kinds of entry that grow by chunks of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextKind {
    Prompt,
    Message,
    Thought,
}

impl EntryKind {
    /// The text a chunk of `kind` extends, when this entry is of that kind.
    pub(crate) const fn text_mut(&mut self, kind: TextKind) -> Option<&mut String> {
        match (self, kind) {
            (Self::Prompt(prompt), TextKind::Prompt) => Some(&mut prompt.text),
            (Self::Steer { text, .. }, TextKind::Prompt)
            | (Self::Message { text }, TextKind::Message)
            | (Self::Thought { text }, TextKind::Thought) => Some(text),
            (
                Self::Ready { .. }
                | Self::Prompt(_)
                | Self::Steer { .. }
                | Self::Question { .. }
                | Self::Message { .. }
                | Self::Thought { .. }
                | Self::Tool(_)
                | Self::Plan { .. }
                | Self::Permission(_)
                | Self::TurnEnded { .. }
                | Self::TurnFailed { .. }
                | Self::Log { .. }
                | Self::Status { .. }
                | Self::Parking(_)
                | Self::Forked { .. }
                | Self::Profile { .. }
                | Self::Model(_),
                TextKind::Prompt | TextKind::Message | TextKind::Thought,
            ) => None,
        }
    }

    /// The tool call, when this is one.
    #[must_use]
    pub const fn as_tool(&self) -> Option<&ToolView> {
        if let Self::Tool(tool) = self {
            Some(tool)
        } else {
            None
        }
    }

    /// The permission request, when this is one.
    #[must_use]
    pub const fn as_permission(&self) -> Option<&Permission> {
        if let Self::Permission(permission) = self {
            Some(permission)
        } else {
            None
        }
    }

    /// Whether this is a tool call or a thought: what "Collapse tool
    /// calls" folds into a run.
    #[must_use]
    pub const fn folds(&self) -> bool {
        matches!(self, Self::Tool(_) | Self::Thought { .. })
    }
}

/// A plan's progress: steps done, steps in all.
#[must_use]
pub fn plan_progress(entries: &[PlanEntry]) -> (usize, usize) {
    let done = entries
        .iter()
        .filter(|entry| entry.status == PlanStatus::Completed)
        .count();
    (done, entries.len())
}

/// A content block as text: a link as Markdown, a file by its name in
/// brackets, anything else as nothing.
#[must_use]
pub fn block_text(block: &ContentBlock) -> String {
    match block {
        ContentBlock::Text { text, .. } => text.clone(),
        ContentBlock::ResourceLink { uri, name, .. } => format!("[{name}]({uri})"),
        ContentBlock::Opaque { .. } => String::new(),
        ContentBlock::Attachment { attachment, .. } => format!("[{}]", attachment.name),
    }
}

/// A prompt's words (its text and links) and its files, apart.
#[must_use]
pub fn prompt_parts(content: &[ContentBlock]) -> (String, Vec<AttachmentRef>) {
    let mut text = String::new();
    let mut attachments = Vec::new();
    for block in content {
        match block {
            ContentBlock::Attachment { attachment, .. } => attachments.push(attachment.clone()),
            ContentBlock::Text { .. }
            | ContentBlock::ResourceLink { .. }
            | ContentBlock::Opaque { .. } => text.push_str(&block_text(block)),
        }
    }
    (text, attachments)
}

/// What happened to a session's agent while the session stayed live
/// (`parked`, `woken`; protocol 12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParkingChange {
    /// Its process was stopped, the conversation kept; why.
    Parked(ParkReason),
    /// It was started again with its conversation.
    Woken,
}

/// Whether the message at `index` is the first reply of its turn: no
/// reply since the person's last message (the reply that carries the
/// agent's circle beside it, slice M).
#[must_use]
pub fn first_reply(entries: &[Entry], index: usize) -> bool {
    entries.get(..index).is_none_or(|before| {
        before
            .iter()
            .rev()
            .find(|entry| {
                matches!(
                    entry.kind,
                    EntryKind::Message { .. } | EntryKind::Prompt(_) | EntryKind::Steer { .. }
                )
            })
            .is_none_or(|entry| !matches!(entry.kind, EntryKind::Message { .. }))
    })
}
