//! The command palette's entries (the page's `data/commands.ts`, and the
//! search its kit's palette runs, `ui/kit/command.tsx`): actions, places,
//! machines, sessions and the theme, in the page's groups and order.
//!
//! [Decision Log] Settings' sections and the theme's choices come from the
//! caller
//! - Purpose: the palette lists each section of Settings and the three
//!   themes, but the sections are the settings frame's and the theme's
//!   choice is the kit's; a second list here would drift from theirs.
//! - Choice: the entries are generic over the section (`S`) and the
//!   theme's choice (`T`); the app passes the frame's sections with their
//!   names and the kit's three choices (system, light, dark), and acts on
//!   the target it gets back.

mod cursor;
mod ranking;
mod subtitle;

pub use subtitle::{DisplayPath, MarkedSubtitle, Subtitle, TextPart};

pub use cursor::{Direction, PaletteCursor};

use std::cmp::Reverse;
use std::ops::Range;

use chrono::{DateTime, Utc};
use vessel_proto::SessionId;

use crate::board::{Fleet, Reach};
use crate::matcher;
use crate::text::Lang;

/// The palette's groups, in the order it lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaletteGroup {
    /// A new session, adding a machine.
    Actions,
    /// The board and Settings' sections.
    GoTo,
    /// Each machine in the book.
    Machines,
    /// Each session, live first.
    Sessions,
    /// The theme's three choices.
    Theme,
}

impl PaletteGroup {
    /// Every group, in the palette's order.
    pub const ALL: [Self; 5] = [
        Self::Actions,
        Self::GoTo,
        Self::Machines,
        Self::Sessions,
        Self::Theme,
    ];
}

/// What choosing an entry does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteTarget<S, T> {
    /// Start a new session, on this machine when one is chosen.
    NewSession(Option<String>),
    /// Add a machine.
    AddMachine,
    /// Go to the board.
    Board,
    /// Go to a section of Settings.
    Settings(S),
    /// Open a machine's detail in Settings › Machines.
    Machine(String),
    /// Open a session.
    Session {
        /// Its machine.
        machine: String,
        /// The session.
        session: SessionId,
        /// Whether it is a conversation (else a terminal), for its icon.
        structured: bool,
    },
    /// Choose the theme.
    Theme(T),
}

/// One entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry<S, T> {
    /// Unique in one listing.
    pub id: String,
    /// What it says.
    pub label: String,
    /// Muted, after the label: a session's state and folder.
    pub detail: Option<Subtitle>,
    /// Searched with the label, not shown (a session's full id).
    pub keywords: Option<String>,
    /// Its group.
    pub group: PaletteGroup,
    /// What choosing it does.
    pub target: PaletteTarget<S, T>,
    /// The node's last activity time, used to rank session navigation.
    /// Commands have no activity time.
    pub activity_at: Option<DateTime<Utc>>,
}

impl<S, T> PaletteEntry<S, T> {
    fn new(id: String, label: String, group: PaletteGroup, target: PaletteTarget<S, T>) -> Self {
        Self {
            id,
            label,
            detail: None,
            keywords: None,
            group,
            target,
            activity_at: None,
        }
    }

    /// Whether every query token matches its visible fields or keywords,
    /// with subsequences confined to a word (Paseo's Command Center).
    #[must_use]
    pub fn found(&self, query: &str) -> bool {
        self.score(&matcher::Query::new(query)).is_some()
    }

    fn score(&self, query: &matcher::Query) -> Option<matcher::Score> {
        let detail = self
            .detail
            .as_ref()
            .map(Subtitle::search_text)
            .unwrap_or_default();
        query.score(&[&self.label, &detail], self.keywords.as_deref())
    }
}

/// Everything the palette lists for `fleet`, `selected` being the machine
/// a new session starts on; `sections` Settings' sections with their
/// names, `themes` the theme's choices as system, light and dark. Live
/// sessions come before ended ones, each newest first, so the one being
/// worked on is near the top.
#[must_use]
pub fn palette_entries<S: Copy, T: Copy>(
    fleet: &Fleet,
    selected: Option<&str>,
    sections: &[(S, &str)],
    themes: [T; 3],
    lang: Lang,
) -> Vec<PaletteEntry<S, T>> {
    let words = lang.palette();
    let actions = [
        PaletteEntry::new(
            "new-session".to_owned(),
            words.new_session.to_owned(),
            PaletteGroup::Actions,
            PaletteTarget::NewSession(selected.map(str::to_owned)),
        ),
        PaletteEntry::new(
            "add-machine".to_owned(),
            words.add_machine.to_owned(),
            PaletteGroup::Actions,
            PaletteTarget::AddMachine,
        ),
    ];
    let board = PaletteEntry::new(
        "go-board".to_owned(),
        words.board.to_owned(),
        PaletteGroup::GoTo,
        PaletteTarget::Board,
    );
    let places = sections.iter().enumerate().map(|(at, (section, name))| {
        PaletteEntry::new(
            format!("go-settings-{at}"),
            words.settings(name),
            PaletteGroup::GoTo,
            PaletteTarget::Settings(*section),
        )
    });
    let sidebar = lang.sidebar();
    let machines = fleet.machines.iter().map(|machine| PaletteEntry {
        detail: Some(Subtitle::Plain(
            match machine.reach {
                Reach::Online => sidebar.connected,
                Reach::Unreachable(_) => sidebar.offline,
            }
            .to_owned(),
        )),
        ..PaletteEntry::new(
            format!("machine-{}", machine.name),
            machine.name.clone(),
            PaletteGroup::Machines,
            PaletteTarget::Machine(machine.name.clone()),
        )
    });
    let mut cards: Vec<_> = fleet
        .machines
        .iter()
        .flat_map(|machine| &machine.sessions)
        .filter(|card| card.detail.marks.archived_at.is_none())
        .collect();
    cards.sort_by_key(|card| Reverse(card.detail.created_at));
    cards.sort_by_key(|card| !card.live);
    let text = lang.text();
    let sessions = cards.into_iter().map(|card| PaletteEntry {
        activity_at: Some(card.detail.active_at),
        detail: Some(Subtitle::StatusPath {
            status: text.phrase(&card.detail.process),
            path: DisplayPath::new(card.detail.cwd.as_deref().unwrap_or("~")),
        }),
        keywords: Some(card.session.to_string()),
        ..PaletteEntry::new(
            format!("session-{}-{}", card.machine, card.session),
            card.title.clone(),
            PaletteGroup::Sessions,
            PaletteTarget::Session {
                machine: card.machine.clone(),
                session: card.session,
                structured: card.structured,
            },
        )
    });
    let themes = themes.into_iter().enumerate().map(|(at, theme)| {
        PaletteEntry::new(
            format!("theme-{at}"),
            words.theme(at),
            PaletteGroup::Theme,
            PaletteTarget::Theme(theme),
        )
    });
    actions
        .into_iter()
        .chain(std::iter::once(board))
        .chain(places)
        .chain(machines)
        .chain(sessions)
        .chain(themes)
        .collect()
}

/// The entries that answer `query`, under their groups in the palette's
/// order; a group with none is left out.
#[must_use]
pub fn palette_groups<'a, S, T>(
    entries: &'a [PaletteEntry<S, T>],
    query: &str,
) -> Vec<(PaletteGroup, Vec<&'a PaletteEntry<S, T>>)> {
    ranking::groups(entries, query)
}

/// Every matching token or subsequence run, in original UTF-8 byte ranges
/// (Paseo's `findHighlightRanges`). Overlapping terms share one mark.
#[must_use]
pub fn marks(query: &str, text: &str) -> Vec<Range<usize>> {
    matcher::Query::new(query).marks(text)
}

/// The row the keyboard makes active after a search changed the rows
/// (Paseo's `preserveActiveResultId`): the same one while it is still
/// listed, else the first.
#[must_use]
pub fn kept<'a>(active: Option<&str>, ids: &[&'a str]) -> Option<&'a str> {
    ids.iter()
        .copied()
        .find(|id| Some(*id) == active)
        .or_else(|| ids.first().copied())
}

/// The row an arrow key makes active (Paseo's `moveActiveResultId`): the
/// next or the previous, round from the last to the first and back; from
/// none, the first going down and the last going up.
#[must_use]
pub fn moved<'a>(active: Option<&str>, ids: &[&'a str], forward: bool) -> Option<&'a str> {
    let last = ids.len().checked_sub(1)?;
    let current = active.and_then(|active| ids.iter().position(|id| *id == active));
    let next = match (current, forward) {
        (None, true) => 0,
        (None, false) => last,
        (Some(at), true) if at >= last => 0,
        (Some(at), true) => at.saturating_add(1),
        (Some(at), false) => at.checked_sub(1).unwrap_or(last),
    };
    ids.get(next).copied()
}
