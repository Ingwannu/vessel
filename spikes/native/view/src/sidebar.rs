//! What the sidebar lists (the page's `data/sidebar.ts`): sessions by
//! machine or by what they are doing, in the order the person chose, each
//! with the mark that says whether it waits on its owner or its agent
//! works.
//!
//! [Decision Log] The sidebar's view, as the page has it (milestone 11,
//! track W; ported in milestone 12, track S)
//! - Grouping by machine (the default) or by status (Paseo's buckets in
//!   the board's words: Needs you, Working, Idle, Ended); sorting by
//!   creation (the default) or by last activity; ended sessions listed
//!   unless the person hides them (Paseo's; the page hid them by default,
//!   see `prefs::SidebarView::default`). An archived session is never
//!   listed; a pinned one comes first in its group.
//! - The flat order the sidebar shows is what "session 1 to 9" and
//!   "next/previous session" walk (`Fleet::sidebar_order`), so a key goes
//!   where the eye already is.
//! - A machine is connected or not: the link reads the book's machines
//!   whole on each refresh and keeps no connection open, so the page's
//!   Connecting, Reconnecting and Draining are not known here yet.
//! - What the person chose in the Display menu is the preferences'
//!   (`prefs::SidebarView`, track E's one model), kept and changed there.

use serde::{Deserialize, Serialize};
use vessel_proto::SessionId;

use crate::board::{Card, Fleet, Machine, Reach};
use crate::prefs::{Grouping, SessionSort, SidebarView};

pub mod display;

/// How many of a machine's sessions the sidebar lists before "N more":
/// the sidebar is a way to switch, not a second board.
pub const SIDEBAR_SESSIONS: usize = 5;

/// How many of a status group's (agent-orchestrator's section limit).
pub const SIDEBAR_STATUS_SESSIONS: usize = 10;

/// The mark a session's row leads with, in the order the status groups
/// are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionMark {
    /// It waits on its owner: a filled dot in the needs-you colour.
    NeedsYou,
    /// Its agent works: a spinner.
    Working,
    /// Live, nothing turning: a hollow dot.
    Idle,
    /// It has ended: a faint dash.
    Ended,
}

impl SessionMark {
    /// Every mark, in the status groups' order.
    pub const ALL: [Self; 4] = [Self::NeedsYou, Self::Working, Self::Idle, Self::Ended];
}

impl Card {
    /// Its row's mark. A session of an older node is idle while it runs,
    /// since nothing says what it is doing.
    #[must_use]
    pub fn session_mark(&self) -> SessionMark {
        if !self.live {
            SessionMark::Ended
        } else if self.needs_you() {
            SessionMark::NeedsYou
        } else if self.is_working() {
            SessionMark::Working
        } else {
            SessionMark::Idle
        }
    }
}

/// A machine's row and the sessions under it.
#[derive(Debug, Clone)]
pub struct SidebarMachine<'a> {
    /// The machine.
    pub machine: &'a Machine,
    /// Whether it answered (its dot; else a dash where the count goes).
    pub connected: bool,
    /// How many live sessions it has.
    pub live: usize,
    /// The first of the listed ones.
    pub sessions: Vec<&'a Card>,
    /// How many more there are than `sessions` shows.
    pub more: usize,
}

/// One status group and its sessions (grouping by status).
#[derive(Debug, Clone)]
pub struct SidebarStatus<'a> {
    /// What they are doing.
    pub mark: SessionMark,
    /// The first of them, each naming its machine.
    pub sessions: Vec<&'a Card>,
    /// How many more there are.
    pub more: usize,
}

/// `cards` as the view lists them: ended ones only when asked for,
/// archived ones never, pinned ones first, then newest first by the
/// chosen time (a stable sort: equal times keep their order).
#[must_use]
pub fn listed<'a>(cards: impl IntoIterator<Item = &'a Card>, view: SidebarView) -> Vec<&'a Card> {
    let mut shown: Vec<&Card> = cards
        .into_iter()
        .filter(|card| (view.ended || card.live) && card.detail.marks.archived_at.is_none())
        .collect();
    let at = |card: &Card| match view.sort {
        SessionSort::Created => card.detail.created_at,
        SessionSort::Activity => card.detail.active_at,
    };
    shown.sort_by(|one, two| {
        two.detail
            .marks
            .pinned
            .cmp(&one.detail.marks.pinned)
            .then(at(two).cmp(&at(one)))
    });
    shown
}

impl Fleet {
    /// Every machine of the book, in its order, with its first `limit`
    /// listed sessions.
    #[must_use]
    pub fn sidebar_machines(&self, view: SidebarView, limit: usize) -> Vec<SidebarMachine<'_>> {
        self.machines
            .iter()
            .map(|machine| {
                let shown = listed(&machine.sessions, view);
                SidebarMachine {
                    machine,
                    connected: machine.reach == Reach::Online,
                    live: machine.sessions.iter().filter(|card| card.live).count(),
                    more: shown.len().saturating_sub(limit),
                    sessions: shown.into_iter().take(limit).collect(),
                }
            })
            .collect()
    }

    /// Every listed session of every machine, grouped by what it is
    /// doing, `limit` to a group; a group with none is left out.
    #[must_use]
    pub fn sidebar_statuses(&self, view: SidebarView, limit: usize) -> Vec<SidebarStatus<'_>> {
        let shown = listed(
            self.machines.iter().flat_map(|machine| &machine.sessions),
            view,
        );
        SessionMark::ALL
            .into_iter()
            .filter_map(|mark| {
                let rows: Vec<&Card> = shown
                    .iter()
                    .copied()
                    .filter(|card| card.session_mark() == mark)
                    .collect();
                (!rows.is_empty()).then(|| SidebarStatus {
                    mark,
                    more: rows.len().saturating_sub(limit),
                    sessions: rows.into_iter().take(limit).collect(),
                })
            })
            .collect()
    }

    /// The sessions as the sidebar shows them, top to bottom: what
    /// "session 1 to 9" counts and "next" and "previous" walk.
    #[must_use]
    pub fn sidebar_order(&self, view: SidebarView) -> Vec<&Card> {
        match view.group {
            Grouping::Machine => self
                .sidebar_machines(view, SIDEBAR_SESSIONS)
                .into_iter()
                .flat_map(|machine| machine.sessions)
                .collect(),
            Grouping::Status => self
                .sidebar_statuses(view, SIDEBAR_STATUS_SESSIONS)
                .into_iter()
                .flat_map(|group| group.sessions)
                .collect(),
        }
    }
}

/// Which way "next" or "previous session" steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Down the list.
    Next,
    /// Up the list.
    Previous,
}

/// The session one `step` from `current` in `order`, wrapping; the first
/// (or the last) when `current` is not in it; `None` for an empty order.
#[must_use]
pub fn step_session<'a>(
    order: &[&'a Card],
    current: Option<(&str, SessionId)>,
    step: Step,
) -> Option<&'a Card> {
    let at = current.and_then(|(machine, session)| {
        order
            .iter()
            .position(|card| card.machine == machine && card.session == session)
    });
    let Some(at) = at else {
        return match step {
            Step::Next => order.first().copied(),
            Step::Previous => order.last().copied(),
        };
    };
    let next = match step {
        Step::Next => at
            .checked_add(1)
            .filter(|next| *next < order.len())
            .unwrap_or(0),
        Step::Previous => at
            .checked_sub(1)
            .unwrap_or_else(|| order.len().saturating_sub(1)),
    };
    order.get(next).copied()
}
