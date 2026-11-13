//! The screen's data, as plain values every surface can draw (and the
//! browser build can read from a file): machines with their sessions, and
//! each session's card on the board.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::{
    DisplayStatus, KanbanColumn, SessionId, SessionInfo, SessionMode, SessionStatus,
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
    /// What it is called (its title, else its folder, else its id).
    pub title: String,
    /// The machine it runs on.
    pub machine: String,
    /// Which agent runs it.
    pub agent: String,
    /// Its lane; `None` for the archive.
    pub lane: Option<Lane>,
    /// The card's status line.
    pub phrase: String,
    /// Whether it waits on its owner (the card leads its lane).
    pub needs_you: bool,
    /// Whether work is turning (the phrase's spinner, the sidebar's dot).
    pub working: bool,
    /// When it last did something.
    pub at: DateTime<Utc>,
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
        let agent = session.spec.harness.to_string();
        let (lane, phrase, needs_you, working, at) = match &session.board {
            Some(board) => (
                lane_of(board.column),
                phrase_of(board.display_status).to_owned(),
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
                    status_word(&session.status),
                    false,
                    matches!(session.status, SessionStatus::Starting),
                    session.ended_at.unwrap_or(session.created_at),
                )
            }
        };
        Self {
            session: session.id.clone(),
            structured: matches!(session.spec.mode, SessionMode::Structured { .. }),
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

    /// "just now", "6m ago", "3h ago", "5d ago" (the page's `sinceLabel`).
    #[must_use]
    pub fn since(&self, now: DateTime<Utc>) -> String {
        let minutes = now.signed_duration_since(self.at).num_minutes();
        if minutes < 1 {
            "just now".to_owned()
        } else if minutes < 60 {
            format!("{minutes}m ago")
        } else if minutes < 60 * 24 {
            format!("{}h ago", minutes / 60)
        } else {
            format!("{}d ago", minutes / (60 * 24))
        }
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

/// The phrase's words (the page's `displayStatusLabel`).
const fn phrase_of(status: DisplayStatus) -> &'static str {
    match status {
        DisplayStatus::Working => "Working",
        DisplayStatus::Blocked => "Blocked",
        DisplayStatus::Exited => "Exited",
        DisplayStatus::NoSignal => "No signal",
        DisplayStatus::Interrupted => "Interrupted",
        DisplayStatus::AwaitingPr => "Awaiting PR",
        DisplayStatus::Idle => "Idle",
        DisplayStatus::FixingCi => "Fixing CI failures",
        DisplayStatus::AddressingComments => "Addressing comments",
        DisplayStatus::NeedsReview => "Needs review",
        DisplayStatus::ReviewScheduled => "Review scheduled",
        DisplayStatus::Reviewing => "Reviewing",
        DisplayStatus::ReviewFailed => "Review failed",
        DisplayStatus::ReviewPending => "Review pending",
        DisplayStatus::Draft => "Draft",
        DisplayStatus::CiFailing => "CI failing",
        DisplayStatus::Commented => "Commented",
        DisplayStatus::ChangesRequested => "Changes requested",
        DisplayStatus::NeedsHumanReview => "Needs human review",
        DisplayStatus::Mergeable => "Mergeable",
        DisplayStatus::Approved => "Approved",
        DisplayStatus::Merged => "Merged",
        DisplayStatus::Closed => "Closed without merge",
        DisplayStatus::Terminated => "Terminated",
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

fn status_word(status: &SessionStatus) -> String {
    match status {
        SessionStatus::Starting => "starting".to_owned(),
        SessionStatus::Running => "running".to_owned(),
        SessionStatus::Exited { code: Some(code) } => format!("exited {code}"),
        SessionStatus::Exited { code: None } => "exited".to_owned(),
        SessionStatus::Failed { .. } => "failed".to_owned(),
        SessionStatus::Interrupted => "interrupted".to_owned(),
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
