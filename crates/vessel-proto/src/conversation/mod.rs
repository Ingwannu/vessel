//! The conversation (protocol 10, milestone 11's Track A): how a prompt is
//! delivered while a turn runs, the queue, attachments, the agent's
//! commands and questions, its usage, and the marks a person puts on a
//! session.
//!
//! Each part is a module of its own; this one gathers what a client reads
//! in one call ([`Conversation`]) and the requests that name a session.
//! The plan's "Track A: contract" section is the reader's summary, and
//! `docs/PROTOCOL.md` the changelog.

mod agent;
mod attachment;
mod marks;
mod question;
mod queue;
mod text;

use serde::{Deserialize, Serialize};

pub use agent::{
    AgentAbilities, AgentCommand, ContextUsage, Cost, FiniteNumber, TurnUsage, UsageObservation,
};
pub use attachment::{
    AttachmentKind, AttachmentLimits, AttachmentRef, BeginUpload, FinishUpload, ReadAttachment,
    Upload, WriteUpload, Written,
};
pub use marks::{
    DeleteSession, KeptMarks, Mark, MarkSession, ResumeSession, SessionDeleted, SessionMarks,
    TidiedWorktree,
};
pub use question::{
    AnswerValue, ChoiceOption, FieldKind, InvalidAnswer, InvalidQuestionPresentation, Question,
    QuestionAnswer, QuestionField, QuestionOutcome, QuestionPresentationError, TextFormat,
};
pub use queue::{
    Delivered, Delivery, PromptQueue, QueueChange, QueueChanged, QueueHold, QueuedPrompt,
};
pub use text::{AttachmentName, CommandName, CostUnit, FieldName, InvalidText, MimeType};

use crate::agent::{ContentBlock, PermissionRequest};
use crate::id::SessionId;

/// A session's conversation as it stands (`session.conversation`): what
/// its log says now, without reading the log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Conversation {
    /// The prompts waiting.
    pub queue: PromptQueue,
    /// The commands the agent offers now (none for an ended session).
    #[serde(default)]
    pub commands: Vec<AgentCommand>,
    /// The agent's last report of its context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<ContextUsage>,
    /// Permission requests waiting for an answer.
    #[serde(default)]
    pub permissions: Vec<PermissionRequest>,
    /// Questions waiting for an answer.
    #[serde(default)]
    pub questions: Vec<Question>,
    /// What the agent can do, once its handshake said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abilities: Option<AgentAbilities>,
    /// How large attachments may be.
    pub limits: AttachmentLimits,
}

/// `session.prompt`: send a prompt, delivered as asked. Needs `operate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PromptSession {
    /// Which session.
    pub session: SessionId,
    /// The message.
    pub content: Vec<ContentBlock>,
    /// How it goes.
    pub delivery: Delivery,
    /// Tell the caller's own session, through its queue, when the turn
    /// this prompt starts waits for an answer and when it finishes, or
    /// the session ends (protocol 11, ADR 0033: the starter told). Only a
    /// session's agent asks, on its own node. Left out when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub notify: bool,
}

/// `session.queue`: change a session's queue. Needs `operate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChangeQueue {
    /// Which session.
    pub session: SessionId,
    /// The change.
    #[serde(flatten)]
    pub change: QueueChange,
}

/// `session.conversation`: a session's conversation as it stands. Needs
/// `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetConversation {
    /// Which session.
    pub session: SessionId,
}
