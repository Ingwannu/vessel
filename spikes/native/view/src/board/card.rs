//! A session's card, made from what its node lists (the page's
//! `placement`, `data/board.ts`; the card's facts, `session-card.tsx`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::{
    Creator, DisplayStatus, EventSeq, HarnessRef, KanbanColumn, SessionInfo, SessionMarks,
    SessionMode, SessionStatus,
};

use super::{Attention, Card, Lane, Phrase, agent_name};
use crate::session::{can_start_again, folder_name};

/// What a card shows besides its place (the page's `session-card.tsx`),
/// and what the sidebar sorts and marks by (`data/sidebar.ts`). Its own
/// struct, with a default, so a card made by hand (the browser's demo)
/// names only what it needs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CardDetail {
    /// Which agent runs it, as the node names it (its mark's letter);
    /// `None` on a card made by hand.
    pub harness: Option<HarnessRef>,
    /// Who started it (the card's origin line).
    pub created_by: Creator,
    /// Its folder, as the node wrote it; `None` is the home folder ("~").
    pub cwd: Option<String>,
    /// Its worktree's branch, when it has one (protocol 10).
    pub branch: Option<String>,
    /// The profile it runs with now (protocol 5).
    pub profile: Option<String>,
    /// The provider's model it runs, when the node says.
    pub model: Option<String>,
    /// Whether it has a title of its own (not its folder's name).
    pub titled: bool,
    /// What its machine marks it (protocol 10): pinned, unread,
    /// archived.
    pub marks: SessionMarks,
    /// Whether "Restore" starts it again (else it only comes back to its
    /// lane): the page's `canStartAgain`.
    pub can_start_again: bool,
    /// Why it could not start, for a failed session (the card's red line).
    pub failure: Option<String>,
    /// Its process's state, whatever the node's phrase (the palette's
    /// detail: the page's `statusWord`).
    pub process: Phrase,
    /// Its pull request as the node watches it (protocol 11): whether
    /// the node can see it and the last one read. Track G1b's.
    #[serde(default)]
    pub pull_request: Option<Box<vessel_proto::forge::SessionPullRequest>>,
    /// When it was created (the sidebar's default order).
    pub created_at: DateTime<Utc>,
    /// When it last did something, by the node's clock where it says, else
    /// its end, else its start (the sidebar's "Last activity").
    pub active_at: DateTime<Utc>,
    /// The record its log will give next (the node's `next_seq`): what the
    /// sidebar's "Mark as read" marks read through, the person saying they
    /// have seen it all without opening it. `None` on a card kept before
    /// this was (the menu then offers no "Mark as read").
    #[serde(default)]
    pub next_seq: Option<EventSeq>,
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
            .or_else(|| folder_name(session.spec.cwd.as_deref()))
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
        let model = session.model.as_ref().map_or_else(
            || match &session.spec.mode {
                SessionMode::Structured { model, .. } => model.as_ref().map(ToString::to_string),
                SessionMode::Terminal { .. } => None,
            },
            |model| Some(model.model.to_string()),
        );
        let detail = CardDetail {
            harness: Some(session.spec.harness.clone()),
            created_by: session.created_by.clone(),
            cwd: session
                .spec
                .cwd
                .as_ref()
                .map(|cwd| cwd.to_string_lossy().into_owned()),
            branch: session
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.branch.as_ref())
                .map(ToString::to_string),
            profile: session
                .profile
                .as_ref()
                .or(session.spec.profile.as_ref())
                .map(ToString::to_string),
            model,
            titled: session.title.is_some(),
            marks: session.marks,
            can_start_again: can_start_again(session),
            failure: match &session.status {
                SessionStatus::Failed { message, .. } => Some(message.clone()),
                SessionStatus::Starting
                | SessionStatus::Running
                | SessionStatus::Exited { .. }
                | SessionStatus::Interrupted => None,
            },
            process: phrase_of_status(&session.status),
            pull_request: session.pull_request.clone(),
            next_seq: Some(session.next_seq),
            created_at: session.created_at,
            active_at: session
                .board
                .as_ref()
                .map(|board| board.last_activity_at)
                .or(session.ended_at)
                .unwrap_or(session.created_at),
        };
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
            detail,
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
        SessionStatus::Exited { code, .. } => Phrase::Exited(*code),
        SessionStatus::Failed { .. } => Phrase::Failed,
        SessionStatus::Interrupted => Phrase::Interrupted,
    }
}
