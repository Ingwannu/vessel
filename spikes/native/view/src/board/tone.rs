//! What a card's status line and its head say beyond the phrase: the
//! colour the line is drawn in (the page's `tone`, `data/phrase.ts`), its
//! agent's mark, its short id, and who started it (`card-origin.tsx`).

use serde::{Deserialize, Serialize};
use vessel_proto::{Creator, DisplayStatus, HarnessKind, HarnessRef};

use super::{Attention, Card, Phrase, Row, agent_name};

/// The colour a card's status line is drawn in: its lane's, unless the
/// phrase says more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tone {
    /// The lane's own colour.
    Lane,
    /// It waits on its owner.
    NeedsYou,
    /// Mergeable or approved.
    Ready,
    /// Merged.
    Merged,
    /// Closed without merge, ended in its lane, or failed in the archive.
    Danger,
    /// Ended, in the archive.
    Quiet,
}

/// Who started a session, when it was not its machine's owner (DESIGN.md
/// §2.4): the card's first line.
#[derive(Debug, Clone)]
pub enum Origin<'a> {
    /// Its machine's owner: the card says nothing.
    Owner,
    /// A paired device, by its id's first six characters.
    Device(String),
    /// Another session's agent.
    Agent {
        /// That session's first four characters.
        id: String,
        /// That session, when it is listed (the line links to it).
        parent: Option<&'a Card>,
        /// Its machine, when it is another than this card's.
        elsewhere: Option<&'a str>,
    },
}

impl Card {
    /// Whether it waits on its owner (the page's `needsYou`): the card
    /// leads its lane, tinted red.
    #[must_use]
    pub fn needs_you(&self) -> bool {
        self.attention == Attention::NeedsYou
    }

    /// Whether work is turning (the page's `isWorking`): a spinner by the
    /// phrase.
    #[must_use]
    pub fn is_working(&self) -> bool {
        self.attention == Attention::Working
    }

    /// The colour of its status line (the page's `tone`).
    #[must_use]
    pub fn tone(&self) -> Tone {
        if self.needs_you() {
            return Tone::NeedsYou;
        }
        if self.lane.is_none() {
            // An ended session's own status word: a failure stays visible
            // in the archive, the rest is quiet.
            return if self.phrase == Phrase::Failed {
                Tone::Danger
            } else {
                Tone::Quiet
            };
        }
        match self.phrase {
            Phrase::Node(status) => node_tone(status),
            Phrase::Starting
            | Phrase::Running
            | Phrase::Exited(_)
            | Phrase::Failed
            | Phrase::Interrupted => Tone::Lane,
        }
    }

    /// The first four characters of its id: what the chrome shows.
    #[must_use]
    pub fn short_id(&self) -> String {
        first(&self.session.to_string(), 4)
    }

    /// The one letter of its agent's mark (the page's `agentMark`).
    #[must_use]
    pub fn letter(&self) -> char {
        self.detail
            .harness
            .as_ref()
            .map_or_else(|| self.agent.chars().next().unwrap_or('?'), agent_mark)
    }

    /// Its folder as the card writes it: the home folder is "~".
    #[must_use]
    pub fn folder(&self) -> &str {
        self.detail.cwd.as_deref().unwrap_or("~")
    }

    /// The card's "who" line after the id: its profile, else its agent,
    /// and its model when the node says (`Claude Code · opus`).
    #[must_use]
    pub fn who(&self) -> String {
        let who = self.detail.profile.as_deref().unwrap_or(&self.agent);
        self.detail
            .model
            .as_ref()
            .map_or_else(|| who.to_owned(), |model| format!("{who} · {model}"))
    }
}

impl<'a> Row<'a> {
    /// Who started it (the page's `CardOrigin`).
    #[must_use]
    pub fn origin(&self) -> Origin<'a> {
        let card = self.card;
        match &card.detail.created_by {
            Creator::Owner => Origin::Owner,
            Creator::Device { id, .. } => Origin::Device(first(&id.to_string(), 6)),
            Creator::Agent { session, .. } => Origin::Agent {
                id: first(&session.to_string(), 4),
                parent: self.parent,
                elsewhere: self
                    .parent
                    .filter(|parent| parent.machine != card.machine)
                    .map(|parent| parent.machine.as_str()),
            },
        }
    }
}

/// The one letter an agent's mark shows: the four by their own letter
/// (Claude Code and Codex both start with C), an added one by its name's
/// first.
#[must_use]
pub fn agent_mark(agent: &HarnessRef) -> char {
    match agent.known() {
        Some(HarnessKind::ClaudeCode) => 'C',
        Some(HarnessKind::Codex) => 'X',
        Some(HarnessKind::Gemini) => 'G',
        Some(HarnessKind::OpenCode) => 'O',
        None => agent_name(agent)
            .chars()
            .next()
            .map_or('?', |letter| letter.to_uppercase().next().unwrap_or(letter)),
    }
}

const fn node_tone(status: DisplayStatus) -> Tone {
    match status {
        DisplayStatus::Mergeable | DisplayStatus::Approved => Tone::Ready,
        DisplayStatus::Merged => Tone::Merged,
        DisplayStatus::Closed | DisplayStatus::Exited => Tone::Danger,
        DisplayStatus::Working
        | DisplayStatus::Blocked
        | DisplayStatus::NoSignal
        | DisplayStatus::Interrupted
        | DisplayStatus::AwaitingPr
        | DisplayStatus::Idle
        | DisplayStatus::FixingCi
        | DisplayStatus::AddressingComments
        | DisplayStatus::NeedsReview
        | DisplayStatus::ReviewScheduled
        | DisplayStatus::Reviewing
        | DisplayStatus::ReviewFailed
        | DisplayStatus::ReviewPending
        | DisplayStatus::Draft
        | DisplayStatus::CiFailing
        | DisplayStatus::Commented
        | DisplayStatus::ChangesRequested
        | DisplayStatus::NeedsHumanReview
        | DisplayStatus::Terminated => Tone::Lane,
    }
}

fn first(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
}
