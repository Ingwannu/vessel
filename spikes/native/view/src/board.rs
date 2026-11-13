//! The screen's data, as plain values every surface can draw (and the
//! browser build can read from a file): machines with their sessions, and
//! each session's card on the board.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::agents::{AgentDetail, AgentInfo, InstallPart};
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
    /// What it is called: its title, else its folder's name, else empty
    /// (the text table says "Untitled session", as the page's `titles.ts`).
    pub title: String,
    /// The machine it runs on.
    pub machine: String,
    /// Which agent runs it.
    pub agent: String,
    /// Its lane; `None` for the archive.
    pub lane: Option<Lane>,
    /// The card's status line.
    pub phrase: Phrase,
    /// Whether it waits on its owner or is working (its dot, its edge, its
    /// place in the lane).
    pub attention: Attention,
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

/// What a session asks of the eye: the page's `needsYou` and `isWorking`,
/// one value since a session is never both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Attention {
    /// Nothing turning, nothing asked.
    AtRest,
    /// Work is turning (the phrase's spinner, the sidebar's blue dot).
    Working,
    /// It waits on its owner (the card leads its lane, a red edge).
    NeedsYou,
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
    /// Why it cannot start a conversation now, in the node's words (an
    /// adapter missing, not signed in); `None` when it can.
    pub unavailable: Option<String>,
}

impl Agent {
    /// Every agent of a machine, the four's installed ones and those added
    /// on it, in the node's order; one that cannot start a conversation
    /// now is kept, with why (the owner: every agent is listed, as Paseo
    /// lists every provider, a disabled one with its reason, none left out
    /// silently).
    #[must_use]
    pub fn all(harnesses: &[Harness]) -> Vec<Self> {
        harnesses
            .iter()
            .map(|harness| Self {
                harness: harness.kind.clone(),
                name: agent_name(&harness.kind),
                unavailable: match &harness.structured {
                    StructuredSupport::Available => None,
                    StructuredSupport::Unavailable { reason } => Some(reason.clone()),
                },
            })
            .collect()
    }

    /// Every agent the node knows (`agent.list`, protocol 11: the four,
    /// installed or not, and those added), each with whether it can start
    /// a conversation: from the installed harnesses (`harness.list`) when
    /// it is one of them, else not installed, said with the install command
    /// the node offers for it (or, for an added agent, its program).
    /// `harness.list` names only what is installed, so without
    /// `agent.list` (an older node) the list is [`Self::all`]'s.
    #[must_use]
    pub fn known(harnesses: &[Harness], agents: &[AgentInfo]) -> Vec<Self> {
        let mut out: Vec<Self> = agents
            .iter()
            .map(|info| {
                let installed = harnesses.iter().find(|harness| harness.kind == info.agent);
                let name = match &info.detail {
                    AgentDetail::Added { added } => added.label.as_ref().map_or_else(
                        || agent_name(&info.agent),
                        |label| label.as_str().to_owned(),
                    ),
                    AgentDetail::Known { .. } => agent_name(&info.agent),
                };
                let unavailable = match installed {
                    Some(harness) => match &harness.structured {
                        StructuredSupport::Available => None,
                        StructuredSupport::Unavailable { reason } => Some(reason.clone()),
                    },
                    None => Some(not_installed(&info.detail)),
                };
                Self {
                    harness: info.agent.clone(),
                    name,
                    unavailable,
                }
            })
            .collect();
        // An installed harness the list left out (a newer kind) still shows.
        for extra in Self::all(harnesses) {
            if !out.iter().any(|agent| agent.harness == extra.harness) {
                out.push(extra);
            }
        }
        out
    }

    /// Whether it can start a conversation now.
    #[must_use]
    pub const fn available(&self) -> bool {
        self.unavailable.is_none()
    }
}

/// Why an agent the node knows is not installed, in the node's manner
/// (its reason for a missing adapter reads the same way).
fn not_installed(detail: &AgentDetail) -> String {
    match detail {
        AgentDetail::Known { installs, .. } => installs
            .iter()
            .find(|offer| offer.part == InstallPart::Harness)
            .or_else(|| installs.first())
            .map_or_else(
                || "not installed on this machine".to_owned(),
                |offer| {
                    format!(
                        "not installed on this machine (install it with `{}`)",
                        offer.command
                    )
                },
            ),
        AgentDetail::Added { added } => format!(
            "its program `{}` is not found on this machine",
            added.program.as_str()
        ),
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
        cards.sort_by(|one, two| {
            let needs_you = |card: &Card| card.attention == Attention::NeedsYou;
            needs_you(two)
                .cmp(&needs_you(one))
                .then(two.at.cmp(&one.at))
        });
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
        cards.sort_by_key(|card| std::cmp::Reverse(card.at));
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
            .unwrap_or_default();
        let agent = agent_name(&session.spec.harness);
        let (lane, phrase, attention, at) = session.board.as_ref().map_or_else(
            || {
                // An older node: placed by the process's state.
                let starting = matches!(session.status, SessionStatus::Starting);
                (
                    session.status.is_live().then_some(Lane::Building),
                    phrase_of_status(&session.status),
                    if starting {
                        Attention::Working
                    } else {
                        Attention::AtRest
                    },
                    session.ended_at.unwrap_or(session.created_at),
                )
            },
            |board| {
                (
                    lane_of(board.column),
                    Phrase::Node(board.display_status),
                    attention_of(board.display_status),
                    board.last_activity_at,
                )
            },
        );
        Self {
            session: session.id,
            structured: matches!(session.spec.mode, SessionMode::Structured { .. }),
            live: session.status.is_live(),
            title,
            machine: machine.to_owned(),
            agent,
            lane,
            phrase,
            attention,
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

/// The page's `needsYou` (blocked, failing checks nobody fixes, changes
/// a person requested) and `isWorking` (a turn or a loop turning).
const fn attention_of(status: DisplayStatus) -> Attention {
    match status {
        DisplayStatus::Blocked | DisplayStatus::CiFailing | DisplayStatus::ChangesRequested => {
            Attention::NeedsYou
        }
        DisplayStatus::Working
        | DisplayStatus::FixingCi
        | DisplayStatus::AddressingComments
        | DisplayStatus::Reviewing
        | DisplayStatus::ReviewPending => Attention::Working,
        DisplayStatus::Exited
        | DisplayStatus::NoSignal
        | DisplayStatus::Interrupted
        | DisplayStatus::AwaitingPr
        | DisplayStatus::Idle
        | DisplayStatus::NeedsReview
        | DisplayStatus::ReviewScheduled
        | DisplayStatus::ReviewFailed
        | DisplayStatus::Draft
        | DisplayStatus::Commented
        | DisplayStatus::NeedsHumanReview
        | DisplayStatus::Mergeable
        | DisplayStatus::Approved
        | DisplayStatus::Merged
        | DisplayStatus::Closed
        | DisplayStatus::Terminated => Attention::AtRest,
    }
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
