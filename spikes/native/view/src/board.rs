//! The screen's data, as plain values every surface can draw (and the
//! browser build can read from a file): machines with their sessions, and
//! each session's card on the board.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::{
    DisplayStatus, Harness, HarnessKind, HarnessRef, KanbanColumn, SessionId, SessionInfo,
    SessionMode, SessionStatus, StructuredSupport,
};

/// The board's four lanes (the archive is not drawn on the spike's screen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lane {
    /// Agents at work.
    Building,
    /// Checks running.
    Validating,
    /// Waiting on a review.
    InReview,
    /// Done and mergeable.
    Ready,
}

impl Lane {
    /// Every lane, in the board's order.
    pub const ALL: [Self; 4] = [
        Self::Building,
        Self::Validating,
        Self::InReview,
        Self::Ready,
    ];

    /// The lane's heading, as the page writes it (`laneLabel`).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Building => "Building",
            Self::Validating => "Validating",
            Self::InReview => "In review",
            Self::Ready => "Ready",
        }
    }
}

/// One session's card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    /// The session.
    pub session: SessionId,
    /// Whether it is a conversation (structured) rather than a terminal.
    pub structured: bool,
    /// Whether its process is alive (it takes prompts).
    pub live: bool,
    /// What it is called (its title, else its folder, else its id).
    pub title: String,
    /// The machine it runs on.
    pub machine: String,
    /// Which agent runs it.
    pub agent: String,
    /// Its lane; `None` for the archive.
    pub lane: Option<Lane>,
    /// The card's status line.
    pub phrase: Phrase,
    /// Whether it waits on its owner (the card leads its lane).
    pub needs_you: bool,
    /// Whether work is turning (the phrase's spinner, the sidebar's dot).
    pub working: bool,
    /// When it last did something.
    pub at: DateTime<Utc>,
}

/// A card's status line, as a value; its words are the text table's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phrase {
    /// The node's phrase (protocol 8).
    Node(DisplayStatus),
    /// From an older node, the process's state.
    Starting,
    /// Running.
    Running,
    /// Exited, with its code when the system gave one.
    Exited(Option<i32>),
    /// Could not start.
    Failed,
    /// The node stopped while it was live.
    Interrupted,
}

/// Whether a machine answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reach {
    /// Connected, sessions listed.
    Online,
    /// Could not be reached; why.
    Unreachable(String),
}

/// A machine in the sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Machine {
    /// Its name in the node book.
    pub name: String,
    /// Whether it answered.
    pub reach: Reach,
    /// Its sessions, archive included.
    pub sessions: Vec<Card>,
    /// The agents a conversation can be started with there.
    pub agents: Vec<Agent>,
}

/// An agent a machine can start a conversation with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    /// Which one, as the node names it.
    pub harness: HarnessRef,
    /// Its name for people (the page's `agentName`).
    pub name: String,
}

impl Agent {
    /// The agents among `harnesses` that run a conversation (an ACP
    /// adapter is there); a harness that only runs in a terminal is left
    /// out until the app draws terminals.
    #[must_use]
    pub fn conversational(harnesses: &[Harness]) -> Vec<Self> {
        harnesses
            .iter()
            .filter(|harness| matches!(harness.structured, StructuredSupport::Available))
            .map(|harness| Self {
                harness: harness.kind.clone(),
                name: agent_name(&harness.kind),
            })
            .collect()
    }
}

/// An agent's name for people (the page's `agentName`).
#[must_use]
pub fn agent_name(agent: &HarnessRef) -> String {
    match agent.known() {
        Some(HarnessKind::ClaudeCode) => "Claude Code".to_owned(),
        Some(HarnessKind::Codex) => "Codex".to_owned(),
        Some(HarnessKind::Gemini) => "Gemini".to_owned(),
        Some(HarnessKind::OpenCode) => "OpenCode".to_owned(),
        None => agent.to_string(),
    }
}

/// Everything the screen shows.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Fleet {
    /// The machines, in the book's order.
    pub machines: Vec<Machine>,
    /// When it was read.
    pub read_at: Option<DateTime<Utc>>,
}

impl Fleet {
    /// One lane's cards across every machine: those waiting on their owner
    /// first, then the most recently active (the page's `board`).
    #[must_use]
    pub fn lane(&self, lane: Lane) -> Vec<&Card> {
        let mut cards: Vec<&Card> = self
            .machines
            .iter()
            .flat_map(|machine| machine.sessions.iter())
            .filter(|card| card.lane == Some(lane))
            .collect();
        cards.sort_by(|one, two| two.needs_you.cmp(&one.needs_you).then(two.at.cmp(&one.at)));
        cards
    }

    /// The archive: every session in no lane (ended, archived), newest
    /// first (the page's `board().archive`).
    #[must_use]
    pub fn archive(&self) -> Vec<&Card> {
        let mut cards: Vec<&Card> = self
            .machines
            .iter()
            .flat_map(|machine| machine.sessions.iter())
            .filter(|card| card.lane.is_none())
            .collect();
        cards.sort_by(|one, two| two.at.cmp(&one.at));
        cards
    }

    /// How many sessions the board's lanes show.
    #[must_use]
    pub fn on_board(&self) -> usize {
        self.machines
            .iter()
            .flat_map(|machine| machine.sessions.iter())
            .filter(|card| card.lane.is_some())
            .count()
    }

    /// The machines that answered, for starting a session.
    pub fn online(&self) -> impl Iterator<Item = &Machine> {
        self.machines
            .iter()
            .filter(|machine| machine.reach == Reach::Online)
    }
}

impl Card {
    /// A session's card, placed as the page places it (`placement`): the
    /// node's lane and phrase when it sent one, else its process state.
    #[must_use]
    pub fn from_session(machine: &str, session: &SessionInfo) -> Self {
        let title = session
            .title
            .as_ref()
            .map(ToString::to_string)
            .or_else(|| {
                session
                    .spec
                    .cwd
                    .as_ref()
                    .and_then(|cwd| cwd.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| session.id.to_string());
        let agent = agent_name(&session.spec.harness);
        let (lane, phrase, needs_you, working, at) = match &session.board {
            Some(board) => (
                lane_of(board.column),
                Phrase::Node(board.display_status),
                needs_you(board.display_status),
                working(board.display_status),
                board.last_activity_at,
            ),
            None => {
                let live = matches!(
                    session.status,
                    SessionStatus::Starting | SessionStatus::Running
                );
                (
                    live.then_some(Lane::Building),
                    phrase_of_status(&session.status),
                    false,
                    matches!(session.status, SessionStatus::Starting),
                    session.ended_at.unwrap_or(session.created_at),
                )
            }
        };
        Self {
            session: session.id.clone(),
            structured: matches!(session.spec.mode, SessionMode::Structured { .. }),
            live: session.status.is_live(),
            title,
            machine: machine.to_owned(),
            agent,
            lane,
            phrase,
            needs_you,
            working,
            at,
        }
    }

    /// Whole minutes since it last did something (the text table words
    /// it: the page's `sinceLabel`).
    #[must_use]
    pub fn minutes_since(&self, now: DateTime<Utc>) -> i64 {
        now.signed_duration_since(self.at).num_minutes()
    }
}

const fn lane_of(column: KanbanColumn) -> Option<Lane> {
    match column {
        KanbanColumn::Building => Some(Lane::Building),
        KanbanColumn::Validating => Some(Lane::Validating),
        KanbanColumn::NeedsReview => Some(Lane::InReview),
        KanbanColumn::Ready => Some(Lane::Ready),
        KanbanColumn::Archive => None,
    }
}

const fn needs_you(status: DisplayStatus) -> bool {
    matches!(
        status,
        DisplayStatus::Blocked | DisplayStatus::CiFailing | DisplayStatus::ChangesRequested
    )
}

const fn working(status: DisplayStatus) -> bool {
    matches!(
        status,
        DisplayStatus::Working
            | DisplayStatus::FixingCi
            | DisplayStatus::AddressingComments
            | DisplayStatus::Reviewing
            | DisplayStatus::ReviewPending
    )
}

const fn phrase_of_status(status: &SessionStatus) -> Phrase {
    match status {
        SessionStatus::Starting => Phrase::Starting,
        SessionStatus::Running => Phrase::Running,
        SessionStatus::Exited { code } => Phrase::Exited(*code),
        SessionStatus::Failed { .. } => Phrase::Failed,
        SessionStatus::Interrupted => Phrase::Interrupted,
    }
}

/// `count` rows of made-up text for the scrolling measurement (question
/// 3): the page's figure is a 20,000-entry timeline, so the native list
/// gets 20,000 rows of comparable text, a third of them Hangul.
#[must_use]
pub fn rows_for_scroll(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| match index % 3 {
            0 => format!(
                "#{index} Read crates/vessel-client/src/client.rs and summarised the connect path"
            ),
            1 => format!("#{index} 테스트를 다시 실행했고 실패한 두 건의 원인을 찾았습니다"),
            _ => format!("#{index} Ran cargo test --workspace: 1,284 passed, 0 failed, 3 ignored"),
        })
        .collect()
}
