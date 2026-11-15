//! Searching every machine's messages for the palette's Messages group
//! (`session.search`, protocol 12, ADR 0037; GOAT's
//! `CommandPaletteThreadItems`, its message rows): the query, each
//! machine's pages, and the rows the group shows.
//!
//! [Decision Log]
//! - Purpose: a person finds what was said in any session on any machine
//!   from the palette, and opens the session at that message.
//! - Constraint: each node searches only itself, newest first, a page at a
//!   time; a page may hold fewer hits than asked and still say there is
//!   more (its work ran out first), and then says how far back it read
//!   (`reached`). Machines answer at their own pace; one may be too old to
//!   search (before protocol 12), unreachable, or refuse.
//! - The reference: GOAT asks every connected environment, ranks the
//!   matches, shows at most ten, and keeps the rows already shown in their
//!   places when a slower environment answers.
//! - Choice: rows newest first across machines, at most [`PAGE`] to
//!   start and [`PAGE`] more each time the person asks; a hit older than
//!   the newest `reached` of a machine that has more is held back until
//!   that machine's next page (its own newer matches may still come), as
//!   the protocol asks of a client that merges machines; a row once shown
//!   keeps its place while others arrive. Each machine's state is its own
//!   ([`MachineState`]), so a slow, old or refusing machine is said once
//!   and holds back no other.
//! - Cost: ranking is by time only (the node's order); GOAT also ranks by
//!   how recently a thread was used, which the app does not record.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use vessel_proto::search::{
    SearchCursor, SearchHit, SearchLimit, SearchMessages, SearchPage, SearchQuery,
};
use vessel_proto::{EventSeq, SessionId};

/// Rows shown at first, and added each time the person asks for more
/// (GOAT's `PALETTE_MESSAGE_LIMIT`).
pub const PAGE: usize = 10;

/// One message row: the machine, the session and the record where the
/// message starts (a hit's identity across pages and answers).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HitId {
    /// The machine's name in the book.
    pub machine: String,
    /// The session.
    pub session: SessionId,
    /// The record where the message starts.
    pub seq: EventSeq,
}

/// A row of the Messages group, top to bottom.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Row {
    /// A message that matched.
    Hit(HitId),
    /// More messages may match: show them.
    More,
}

/// Safe facts retained independently of the selected display language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFailure {
    /// The bounded request did not complete.
    TimedOut,
    /// A peer or local command queue refused the search.
    Failed,
}

/// Where one machine's search stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineState {
    /// A page was asked for and has not come.
    Asking,
    /// Older messages may still match: the next page starts at `next`;
    /// every match newer than `reached` is in the pages read.
    More {
        /// Where the next page starts.
        next: SearchCursor,
        /// How far back the pages read went.
        reached: Option<DateTime<Utc>>,
    },
    /// Every match is in the pages read.
    Done,
    /// The machine speaks a protocol before 12: it cannot search.
    Older,
    /// The machine refused or could not answer: a safe, untranslated fact.
    Failed(SearchFailure),
}

/// One machine's hits and state.
#[derive(Debug, Clone)]
struct Machine {
    name: String,
    hits: Vec<SearchHit>,
    state: MachineState,
}

/// How a machine answered a page.
#[derive(Debug, Clone)]
pub enum Answer {
    /// Its page.
    Page(SearchPage),
    /// It cannot search (before protocol 12).
    Older,
    /// It is not connected: it is left out, said nowhere (the sidebar
    /// says it is offline).
    NotConnected,
    /// It refused, or the connection failed: a safe, untranslated fact.
    Failed(SearchFailure),
}

/// What the group says under its rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// The words are not a search the node takes (too long, too many
    /// words, a control character).
    Invalid,
    /// No row yet and a machine has not answered.
    Searching,
    /// Every machine answered and none has a match.
    Nothing,
    /// These machines are too old to search.
    Older(Vec<String>),
    /// These machines could not search: each with its safe failure kind.
    Failed(Vec<(String, SearchFailure)>),
}

/// A page to ask a machine for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageAsk {
    /// The machine.
    pub machine: String,
    /// The request.
    pub request: SearchMessages,
}

/// The palette's message search.
#[derive(Debug, Clone, Default)]
pub struct MessageSearch {
    /// The words searched; `None`: no search (the field is empty).
    query: Option<SearchQuery>,
    /// The words were not a search.
    invalid: bool,
    /// Searches started, so an answer to an earlier one is dropped.
    ticket: u64,
    machines: Vec<Machine>,
    /// How many rows the person wants.
    want: usize,
    /// The rows shown, in their places.
    shown: Vec<HitId>,
}

impl MessageSearch {
    /// Search `text` on `machines` (the reachable ones, in the book's
    /// order): the pages to ask for, each tagged with [`Self::ticket`].
    /// Empty text ends the search; text the node would refuse is said
    /// ([`Note::Invalid`]) and asks nothing.
    pub fn start<'a>(
        &mut self,
        text: &str,
        machines: impl IntoIterator<Item = &'a str>,
    ) -> Vec<PageAsk> {
        self.ticket = self.ticket.saturating_add(1);
        self.machines.clear();
        self.shown.clear();
        self.want = PAGE;
        self.invalid = false;
        self.query = None;
        if text.trim().is_empty() {
            return Vec::new();
        }
        let Ok(query) = text.parse::<SearchQuery>() else {
            self.invalid = true;
            return Vec::new();
        };
        self.machines = machines
            .into_iter()
            .map(|name| Machine {
                name: name.to_owned(),
                hits: Vec::new(),
                state: MachineState::Asking,
            })
            .collect();
        let asks = self
            .machines
            .iter()
            .map(|machine| PageAsk {
                machine: machine.name.clone(),
                request: request(&query, None),
            })
            .collect();
        self.query = Some(query);
        asks
    }

    /// The search the answers belong to.
    #[must_use]
    pub const fn ticket(&self) -> u64 {
        self.ticket
    }

    /// The words searched, once they are a search.
    #[must_use]
    pub fn query(&self) -> Option<&str> {
        self.query.as_ref().map(SearchQuery::as_str)
    }

    /// Retain the validated query before closing the palette and clearing results.
    #[must_use]
    pub const fn query_value(&self) -> Option<&SearchQuery> {
        self.query.as_ref()
    }

    /// Whether a search is under way or shown (the group is drawn).
    #[must_use]
    pub const fn active(&self) -> bool {
        self.query.is_some() || self.invalid
    }

    /// `machine` answered search `ticket`: its page joins the rows (an
    /// answer to an earlier search, or from a machine not asked, is
    /// dropped). Returns the pages to ask for next: when fewer rows match
    /// than the person wants and a machine has more.
    pub fn answered(&mut self, ticket: u64, machine: &str, answer: Answer) -> Vec<PageAsk> {
        if ticket != self.ticket {
            return Vec::new();
        }
        let Some(slot) = self.machines.iter_mut().find(|known| known.name == machine) else {
            return Vec::new();
        };
        match answer {
            Answer::Page(page) => {
                slot.hits.extend(page.hits);
                slot.state = match page.next {
                    Some(next) => MachineState::More {
                        next,
                        reached: page.reached,
                    },
                    None => MachineState::Done,
                };
            }
            Answer::Older => slot.state = MachineState::Older,
            Answer::NotConnected => slot.state = MachineState::Done,
            Answer::Failed(why) => slot.state = MachineState::Failed(why),
        }
        self.arrange();
        self.pages_wanted()
    }

    /// The person asked for more rows: the pages to ask for.
    pub fn more(&mut self) -> Vec<PageAsk> {
        self.want = self.want.saturating_add(PAGE);
        self.arrange();
        self.pages_wanted()
    }

    /// The rows, top to bottom: the hits shown, then More when more may
    /// match.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        let mut rows: Vec<Row> = self.shown.iter().cloned().map(Row::Hit).collect();
        if self.has_more() {
            rows.push(Row::More);
        }
        rows
    }

    /// The hit a row names.
    #[must_use]
    pub fn hit(&self, id: &HitId) -> Option<&SearchHit> {
        self.machines
            .iter()
            .find(|machine| machine.name == id.machine)?
            .hits
            .iter()
            .find(|hit| hit.session == id.session && hit.seq == id.seq)
    }

    /// Each machine's state, in the book's order.
    pub fn states(&self) -> impl Iterator<Item = (&str, &MachineState)> {
        self.machines
            .iter()
            .map(|machine| (machine.name.as_str(), &machine.state))
    }

    /// What the group says under its rows, in order.
    #[must_use]
    pub fn notes(&self) -> Vec<Note> {
        if self.invalid {
            return vec![Note::Invalid];
        }
        if self.query.is_none() {
            return Vec::new();
        }
        let mut notes = Vec::new();
        let asking = self
            .machines
            .iter()
            .any(|machine| machine.state == MachineState::Asking);
        if self.shown.is_empty() && asking {
            notes.push(Note::Searching);
        } else if self.shown.is_empty() && !self.has_more() {
            notes.push(Note::Nothing);
        }
        let older: Vec<String> = self
            .machines
            .iter()
            .filter(|machine| machine.state == MachineState::Older)
            .map(|machine| machine.name.clone())
            .collect();
        if !older.is_empty() {
            notes.push(Note::Older(older));
        }
        let failed: Vec<(String, SearchFailure)> = self
            .machines
            .iter()
            .filter_map(|machine| match &machine.state {
                MachineState::Failed(why) => Some((machine.name.clone(), *why)),
                MachineState::Asking
                | MachineState::More { .. }
                | MachineState::Done
                | MachineState::Older => None,
            })
            .collect();
        if !failed.is_empty() {
            notes.push(Note::Failed(failed));
        }
        notes
    }

    /// The newest `reached` among machines with more to read: a hit older
    /// than it may yet be outranked by that machine's next page.
    fn floor(&self) -> Option<DateTime<Utc>> {
        self.machines
            .iter()
            .filter_map(|machine| match &machine.state {
                MachineState::More { reached, .. } => *reached,
                MachineState::Asking
                | MachineState::Done
                | MachineState::Older
                | MachineState::Failed(_) => None,
            })
            .max()
    }

    /// Every hit that may show now, newest first.
    fn ranked(&self) -> Vec<HitId> {
        let floor = self.floor();
        let mut hits: Vec<(&str, &SearchHit)> = self
            .machines
            .iter()
            .flat_map(|machine| machine.hits.iter().map(|hit| (machine.name.as_str(), hit)))
            .filter(|(_, hit)| floor.is_none_or(|floor| hit.at >= floor))
            .collect();
        hits.sort_by(|(left_machine, left), (right_machine, right)| {
            right
                .at
                .cmp(&left.at)
                .then_with(|| left_machine.cmp(right_machine))
                .then_with(|| right.seq.cmp(&left.seq))
        });
        hits.into_iter()
            .map(|(machine, hit)| HitId {
                machine: machine.to_owned(),
                session: hit.session,
                seq: hit.seq,
            })
            .collect()
    }

    /// The rows shown again: the ones already shown stay in their places,
    /// new ones follow in rank, up to what the person wants.
    fn arrange(&mut self) {
        let ranked = self.ranked();
        let present: HashSet<&HitId> = ranked.iter().collect();
        let mut shown: Vec<HitId> = self
            .shown
            .iter()
            .filter(|id| present.contains(id))
            .cloned()
            .collect();
        let kept: HashSet<HitId> = shown.iter().cloned().collect();
        for id in ranked {
            if shown.len() >= self.want {
                break;
            }
            if !kept.contains(&id) {
                shown.push(id);
            }
        }
        self.shown = shown;
    }

    /// Whether more rows may match than are shown.
    fn has_more(&self) -> bool {
        self.ranked().len() > self.shown.len()
            || self
                .machines
                .iter()
                .any(|machine| matches!(machine.state, MachineState::More { .. }))
    }

    /// The next pages to ask for: when fewer rows show than are wanted,
    /// each machine with more that is not already being asked.
    fn pages_wanted(&mut self) -> Vec<PageAsk> {
        if self.shown.len() >= self.want {
            return Vec::new();
        }
        let Some(query) = self.query.clone() else {
            return Vec::new();
        };
        let mut asks = Vec::new();
        for machine in &mut self.machines {
            if let MachineState::More { next, .. } = machine.state {
                asks.push(PageAsk {
                    machine: machine.name.clone(),
                    request: request(&query, Some(next)),
                });
                machine.state = MachineState::Asking;
            }
        }
        asks
    }
}

/// A page's request: every session, [`PAGE`] hits.
fn request(query: &SearchQuery, before: Option<SearchCursor>) -> SearchMessages {
    SearchMessages {
        query: query.clone(),
        session: None,
        before,
        limit: u8::try_from(PAGE).ok().and_then(SearchLimit::new),
    }
}
