//! The user prompts of a session's log, in order (TRACKER 803, ADR 0110):
//! what a reload lines up with the user messages of the agent's own
//! history before it adds anything to the log.
//!
//! [Decision Log]
//! - Purpose: a reload adds only what the agent's history has beyond the
//!   log, and only when the log's prompts are exactly the first prompts of
//!   that history (the coordinator's ruling: compare user prompts exactly).
//!   Anything else could add a turn the log already has.
//! - What one prompt is, in the log: a turn's prompt (`turn_started`), a
//!   steer (`steered`), and, outside a turn, a run of user message chunks
//!   (an import's or an earlier reload's records of the agent's history).
//!   A user message chunk inside a turn is an agent echoing the prompt it
//!   was sent, which is already counted. In the agent's history: a run of
//!   user message chunks ([`messages`]).
//! - The text of a prompt: its text blocks joined with nothing between
//!   them, on both sides. Links, attachments and other blocks have no text
//!   here: an image the history keeps is not the attachment Vessel sent,
//!   but the words beside it are the same.
//! - Only agent events take part. A record the node adds between two
//!   chunks of one message (a usage report, an options change) does not
//!   split it, as it does not in the agent's history.
//! - Bound: [`UserPrompts::MAX_BYTES`] of text. Past it, or when a stored
//!   log's first records were pruned, the fold says it does not know
//!   (`None`), and a reload then adds nothing.
//! - Who folds: the node, over the stored records before this run (the
//!   ring's `start`), and the publisher, over every record of this run as
//!   it commits it. The two are joined at a run's start, where no turn
//!   runs; so no record is missed or counted twice, whatever arrives
//!   while the node reads the store.

use vessel_proto::{AgentEvent, ContentBlock, SessionEvent};

/// A log's user prompts, folded record by record.
#[derive(Debug, Clone)]
pub struct UserPrompts {
    /// The prompts so far; `None` once they cannot be known.
    prompts: Option<Vec<String>>,
    /// A run of user message chunks outside a turn, not yet ended.
    open: Option<String>,
    /// Inside a turn: chunks there are the prompt's echo.
    in_turn: bool,
    /// Bytes of text held.
    bytes: usize,
}

impl UserPrompts {
    /// The most text a fold holds before it gives up (4 MiB).
    pub const MAX_BYTES: usize = 4 * 1024 * 1024;

    /// An empty fold: a log with no records yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            prompts: Some(Vec::new()),
            open: None,
            in_turn: false,
            bytes: 0,
        }
    }

    /// A fold that cannot know the prompts (a log whose first records were
    /// pruned).
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            prompts: None,
            open: None,
            in_turn: false,
            bytes: 0,
        }
    }

    /// Take in the next record of the log.
    pub fn observe(&mut self, event: &SessionEvent) {
        match event {
            SessionEvent::Agent { event } => self.agent(event),
            SessionEvent::Status { status } if !status.is_live() => {
                self.close();
                self.in_turn = false;
            }
            SessionEvent::Status { .. }
            | SessionEvent::Terminal(_)
            | SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            | SessionEvent::Forked { .. }
            | SessionEvent::Rewound { .. }
            | SessionEvent::Reloaded { .. } => {}
        }
    }

    fn agent(&mut self, event: &AgentEvent) {
        match event {
            AgentEvent::TurnStarted { content } => {
                self.close();
                self.push(text(content));
                self.in_turn = true;
            }
            AgentEvent::Steered { content } => {
                self.close();
                self.push(text(content));
            }
            AgentEvent::UserMessageChunk { content } if !self.in_turn => {
                let chunk = block(content);
                self.weigh(chunk.len());
                if let Some(open) = &mut self.open {
                    open.push_str(chunk);
                } else {
                    self.open = Some(chunk.to_owned());
                }
            }
            AgentEvent::TurnEnded { .. } | AgentEvent::TurnFailed { .. } => {
                self.close();
                self.in_turn = false;
            }
            // An echo inside a turn; the agent's own words and acts end a
            // run of user chunks.
            AgentEvent::UserMessageChunk { .. }
            | AgentEvent::Ready { .. }
            | AgentEvent::MessageChunk { .. }
            | AgentEvent::ThoughtChunk { .. }
            | AgentEvent::ToolCall(_)
            | AgentEvent::ToolCallUpdate(_)
            | AgentEvent::Plan { .. }
            | AgentEvent::PermissionRequested(_)
            | AgentEvent::PermissionResolved { .. }
            | AgentEvent::QuestionAsked(_)
            | AgentEvent::QuestionResolved { .. } => self.close(),
            // The node's own lines are not the conversation's.
            AgentEvent::HarnessLog { .. } => {}
        }
    }

    /// This fold followed by `run`'s, which began where this one ends (a
    /// run's start, outside any turn).
    #[must_use]
    pub fn then(mut self, run: &Self) -> Self {
        self.close();
        let mut run = run.clone();
        run.close();
        self.prompts = match (self.prompts, run.prompts) {
            (Some(mut before), Some(after)) => {
                before.extend(after);
                Some(before)
            }
            (None, _) | (_, None) => None,
        };
        self.bytes = self.bytes.saturating_add(run.bytes);
        if self.bytes > Self::MAX_BYTES {
            self.prompts = None;
        }
        self.in_turn = run.in_turn;
        self
    }

    /// The prompts, an open run of chunks included; `None` when they
    /// cannot be known.
    #[must_use]
    pub fn prompts(&self) -> Option<Vec<&str>> {
        let mut all: Vec<&str> = self.prompts.as_ref()?.iter().map(String::as_str).collect();
        all.extend(self.open.as_deref());
        Some(all)
    }

    fn push(&mut self, prompt: String) {
        self.weigh(prompt.len());
        if let Some(prompts) = &mut self.prompts {
            prompts.push(prompt);
        }
    }

    fn close(&mut self) {
        if let Some(open) = self.open.take()
            && let Some(prompts) = &mut self.prompts
        {
            prompts.push(open);
        }
    }

    fn weigh(&mut self, bytes: usize) {
        self.bytes = self.bytes.saturating_add(bytes);
        if self.bytes > Self::MAX_BYTES {
            self.prompts = None;
            self.open = None;
        }
    }
}

impl Default for UserPrompts {
    fn default() -> Self {
        Self::new()
    }
}

/// The text of a prompt's blocks, joined with nothing between them.
pub(super) fn text(content: &[ContentBlock]) -> String {
    content.iter().map(block).collect()
}

/// A block's text; empty for any block that is not text.
pub(super) fn block(content: &ContentBlock) -> &str {
    match content {
        ContentBlock::Text { text } => text,
        ContentBlock::ResourceLink { .. }
        | ContentBlock::Opaque { .. }
        | ContentBlock::Attachment { .. } => "",
    }
}

/// The user messages of an agent's history (a reload's replay): each run
/// of user message chunks is one, as [`UserPrompts`] counts them outside a
/// turn. Each is its text and the index of its first event.
pub(super) fn messages(replay: &[AgentEvent]) -> Vec<(usize, String)> {
    let mut found: Vec<(usize, String)> = Vec::new();
    let mut open = false;
    for (at, event) in replay.iter().enumerate() {
        if let AgentEvent::UserMessageChunk { content } = event {
            let chunk = block(content);
            if open && let Some((_, text)) = found.last_mut() {
                text.push_str(chunk);
            } else {
                found.push((at, chunk.to_owned()));
            }
            open = true;
        } else if !matches!(event, AgentEvent::HarnessLog { .. }) {
            open = false;
        }
    }
    found
}
