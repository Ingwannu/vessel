//! A structured session's conversation, folded from its records as they
//! arrive: what a person reads, in order.
//!
//! [Decision Log]
//! - Purpose: the session screen shows the conversation growing; the page
//!   folds records in TypeScript (`web/src/data/conversation.ts`), and the
//!   app needs the same fold in Rust, once, for every surface.
//! - Constraint: records arrive one at a time and the fold must cost
//!   little per record (a 20,000-entry conversation is the page's bench).
//! - Alternatives: rebuilding the entry list from every record each time
//!   (quadratic); keeping raw records and folding while drawing (the
//!   screen would then hold protocol types).
//! - Choice: an append-mostly list of entries plus an index from a tool
//!   call's id to its entry, so a chunk extends the last entry and an
//!   update finds its call without a scan.
//! - Cost: only what the first build shows is folded (messages, thoughts,
//!   tool calls by title and state, turn ends, permissions as notices);
//!   plans, questions, attachments and a tool call's content wait for the
//!   real port of the page's reducer, with its tests' cases.

use std::collections::HashMap;

use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, EventSeq, PermissionId, PermissionOption,
    PermissionOptionId, SessionEvent, SessionStatus, ToolCallId, ToolCallStatus,
};

use crate::ConversationWords;
use crate::timeline::{allows, answer_words};

/// Who an entry is from, which decides how it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    /// The person (a prompt).
    Person,
    /// The agent's reply.
    Agent,
    /// The agent's reasoning.
    Thought,
    /// A tool call.
    Tool,
    /// Something that happened (a turn ended, the process stopped).
    Notice,
}

/// A tool call's state, as its entry shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolState {
    /// Announced, not started.
    Pending,
    /// Running.
    Running,
    /// Finished.
    Done,
    /// Failed.
    Failed,
}

impl ToolState {
    /// The word drawn beside its title.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }

    const fn from_status(status: ToolCallStatus) -> Self {
        match status {
            ToolCallStatus::Pending => Self::Pending,
            ToolCallStatus::InProgress => Self::Running,
            ToolCallStatus::Completed => Self::Done,
            ToolCallStatus::Failed => Self::Failed,
        }
    }
}

/// Something that happened, shown in the conversation (its words are the
/// text table's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// The turn failed, and the agent's message.
    TurnFailed(String),
    /// The process ended.
    Ended,
    /// The process failed, and why.
    Failed(String),
    /// The node stopped while the session was live.
    Interrupted,
}

/// One entry of the conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Who it is from.
    pub speaker: Speaker,
    /// Its text (a tool call's title); empty for a notice.
    pub text: String,
    /// A tool call's state; `None` for every other entry.
    pub tool: Option<ToolState>,
    /// What happened, for a notice; `None` for every other entry.
    pub notice: Option<Notice>,
}

/// A permission the agent waits for: what it wants to do, and the
/// answers it offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    /// The request.
    pub id: PermissionId,
    /// What it is for: the tool call's title, when known.
    pub what: Option<String>,
    /// The answers, in the agent's order.
    pub options: Vec<PermissionOption>,
}

impl Asked {
    /// Its buttons, in the agent's order: the option, its words in the
    /// person's language (`timeline::answer_words`), whether it allows.
    #[must_use]
    pub fn buttons(&self, words: &ConversationWords) -> Vec<(PermissionOptionId, String, bool)> {
        self.options
            .iter()
            .zip(answer_words(&self.options, |kind| {
                words.permission_kind(kind)
            }))
            .map(|(option, said)| {
                (
                    option.id.clone(),
                    words.answer_words(said),
                    allows(option.kind),
                )
            })
            .collect()
    }
}

/// The conversation so far.
#[derive(Debug)]
pub struct Conversation {
    entries: Vec<Entry>,
    tools: HashMap<ToolCallId, usize>,
    asked: Vec<Asked>,
    next: EventSeq,
    turn_running: bool,
}

impl Default for Conversation {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            tools: HashMap::new(),
            asked: Vec::new(),
            next: EventSeq::new(0),
            turn_running: false,
        }
    }
}

impl Conversation {
    /// The entries, oldest first.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Where the log continues: the sequence number of the next record.
    #[must_use]
    pub const fn next_seq(&self) -> EventSeq {
        self.next
    }

    /// The permissions waiting for an answer, oldest first.
    #[must_use]
    pub fn asked(&self) -> &[Asked] {
        &self.asked
    }

    /// Whether a turn is running (the composer then queues).
    #[must_use]
    pub const fn turn_running(&self) -> bool {
        self.turn_running
    }

    /// Fold one record in. Records already folded (a resumed stream
    /// repeats none, but a reload might) are ignored by sequence number.
    pub fn apply(&mut self, record: &EventRecord) {
        if record.seq < self.next {
            return;
        }
        // `None` only once u64 is spent: the log is then full and nothing
        // follows, so stopping here loses nothing.
        let Some(next) = record.seq.next() else {
            return;
        };
        self.next = next;
        match &record.event {
            SessionEvent::Agent { event, .. } => self.agent(event),
            SessionEvent::Status { status, .. } => self.status(status),
            SessionEvent::Terminal(_)
            | SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            // Parking (protocol 12, ADR 0038) is not drawn yet.
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            // Protocol 14's fork and rewind records: taken, not drawn yet
            // (the app's rewind and fork are a later slice).
            | SessionEvent::Forked { .. }
            | SessionEvent::Rewound { .. } => {}
        }
    }

    fn agent(&mut self, event: &AgentEvent) {
        match event {
            AgentEvent::TurnStarted { content, .. } | AgentEvent::Steered { content, .. } => {
                self.turn_running = true;
                self.push(Speaker::Person, text_of(content));
            }
            AgentEvent::UserMessageChunk { content, .. } => self.extend(Speaker::Person, content),
            AgentEvent::MessageChunk { content, .. } => self.extend(Speaker::Agent, content),
            AgentEvent::ThoughtChunk { content, .. } => self.extend(Speaker::Thought, content),
            AgentEvent::ToolCall(call) => {
                let index = self.entries.len();
                self.entries.push(Entry {
                    speaker: Speaker::Tool,
                    text: call.title.clone(),
                    tool: Some(ToolState::from_status(call.status)),
                    notice: None,
                });
                let _earlier = self.tools.insert(call.id.clone(), index);
            }
            AgentEvent::ToolCallUpdate(update) => {
                if let Some(entry) = self
                    .tools
                    .get(&update.id)
                    .and_then(|index| self.entries.get_mut(*index))
                {
                    if let Some(title) = &update.title {
                        entry.text.clone_from(title);
                    }
                    if let Some(status) = update.status {
                        entry.tool = Some(ToolState::from_status(status));
                    }
                }
            }
            AgentEvent::PermissionRequested(request) => {
                let what = self
                    .tools
                    .get(&request.tool_call)
                    .and_then(|index| self.entries.get(*index))
                    .map(|entry| entry.text.clone());
                self.asked.push(Asked {
                    id: request.id,
                    what,
                    options: request.options.clone(),
                });
            }
            AgentEvent::PermissionResolved { id, .. } => {
                self.asked.retain(|asked| asked.id != *id);
            }
            AgentEvent::TurnEnded { .. } => {
                self.turn_running = false;
                self.asked.clear();
            }
            AgentEvent::TurnFailed { message, .. } => {
                self.turn_running = false;
                self.asked.clear();
                self.notify(Notice::TurnFailed(message.clone()));
            }
            AgentEvent::Ready { .. }
            | AgentEvent::Plan { .. }
            | AgentEvent::HarnessLog { .. }
            | AgentEvent::QuestionAsked(_)
            | AgentEvent::QuestionResolved { .. } => {}
        }
    }

    fn status(&mut self, status: &SessionStatus) {
        let notice = match status {
            SessionStatus::Starting | SessionStatus::Running => return,
            SessionStatus::Exited { .. } => Notice::Ended,
            SessionStatus::Failed { message, .. } => Notice::Failed(message.clone()),
            SessionStatus::Interrupted => Notice::Interrupted,
        };
        self.turn_running = false;
        self.asked.clear();
        self.notify(notice);
    }

    fn push(&mut self, speaker: Speaker, text: String) {
        self.entries.push(Entry {
            speaker,
            text,
            tool: None,
            notice: None,
        });
    }

    fn notify(&mut self, notice: Notice) {
        self.entries.push(Entry {
            speaker: Speaker::Notice,
            text: String::new(),
            tool: None,
            notice: Some(notice),
        });
    }

    /// A chunk continues the last entry when it is the same speaker's.
    fn extend(&mut self, speaker: Speaker, content: &ContentBlock) {
        let chunk = text_of(std::slice::from_ref(content));
        match self.entries.last_mut() {
            Some(last) if last.speaker == speaker => last.text.push_str(&chunk),
            Some(_) | None => self.push(speaker, chunk),
        }
    }
}

/// The text of a prompt or a chunk; a link by its name, anything else by
/// a placeholder (attachments are not drawn yet).
fn text_of(content: &[ContentBlock]) -> String {
    let mut text = String::new();
    for block in content {
        match block {
            ContentBlock::Text { text: part, .. } => text.push_str(part),
            ContentBlock::ResourceLink { name, .. } => text.push_str(name),
            ContentBlock::Opaque { .. } | ContentBlock::Attachment { .. } => {
                text.push_str("[attachment]");
            }
        }
    }
    text
}
