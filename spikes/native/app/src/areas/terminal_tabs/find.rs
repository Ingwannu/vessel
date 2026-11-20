//! Find in a terminal tab: Paseo's terminal find (`terminal/find`,
//! `pane-find`) over the view's `Finder` (P3).
//!
//! [Decision Log]
//! - Purpose: Mod+F in a terminal tab finds text in its history and screen,
//!   with Paseo's bar, counter and keys (Enter next, Shift+Enter previous,
//!   Escape closes and gives the keyboard back to the terminal).
//! - Shortcut: the pane's key handler hears Mod+F, so it opens this bar only
//!   while the terminal has the keyboard; the conversation's keystroke
//!   observer stands down then (coordinator, P3 question 3).
//! - Search: a slice of rows a step on the window's thread, as the
//!   conversation find does, each step only while the model is home from
//!   its parse; a newer query or new output starts over (a ticket drops
//!   the older run). The current match is the first at or below the view's
//!   top (xterm.js searches on from the viewport); after new output it
//!   stays on the same text when that text still matches.
//! - Reveal: the view scrolls to the current match when it is out of view;
//!   the bar moves to the bottom when it would cover it (Paseo's
//!   `findWidgetCoversSelection`).
//! - Cost: Paseo's find anchor keeps the inspected line still under new
//!   output; here a match scrolled out of view by output is brought back
//!   when the search runs again, a line's movement between.

use std::time::Duration;

use gpui::{Context, Task, Window};
use vessel_app_kit::TextField;
use vessel_app_kit::field::FieldId;
use vessel_app_view::terminal::grid::{Generation, TextAt};
use vessel_app_view::terminal::{
    FIND_LIMIT, Finder, FoundMatch, MAX_FIND_QUERY, first_from, next_of, previous_of,
};

use super::PaneKey;
use crate::shell::Shell;

/// Rows searched a step: a frame's worth at 1,000,000 lines of history.
const ROWS_PER_STEP: usize = 2_000;

/// How long a step waits while the model is away at its parse.
const AWAY: Duration = Duration::from_millis(16);

/// A tab's find, while its bar is open.
pub(crate) struct PaneFind {
    pub(crate) field: TextField,
    query: String,
    /// The search running; `None` once done.
    finder: Option<Finder>,
    /// The last finished search's matches, oldest first.
    matches: Vec<FoundMatch>,
    limited: bool,
    current: Option<usize>,
    /// The current match's first cell, kept across a search again.
    keep: Option<TextAt>,
    /// Which run is current; an older run's task stops at its next step.
    ticket: u64,
    task: Option<Task<()>>,
    /// The grid's change count the last search began at.
    searched: Option<Generation>,
    /// Whether the bar sits at the bottom (it would cover the match).
    pub(crate) bottom: bool,
}

/// What the bar's counter says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Count {
    /// An empty query.
    Nothing,
    /// No match.
    None,
    /// Matches and none current yet.
    Total(String),
    /// The current one of all.
    At(String, String),
}

impl PaneFind {
    /// The counter (Paseo's `terminal/find/index.tsx` 100-106): the total
    /// gets a "+" when it stopped at the limit.
    pub(crate) fn count(&self) -> Count {
        if self.query.is_empty() {
            return Count::Nothing;
        }
        if self.matches.is_empty() {
            return if self.finder.is_some() {
                Count::Nothing
            } else {
                Count::None
            };
        }
        let total = if self.limited {
            format!("{FIND_LIMIT}+")
        } else {
            self.matches.len().to_string()
        };
        match self.current {
            Some(index) => Count::At(index.saturating_add(1).to_string(), total),
            None => Count::Total(total),
        }
    }

    /// Whether previous and next can move.
    pub(crate) const fn can_move(&self) -> bool {
        !self.matches.is_empty()
    }

    /// The matches and the current one, for the frame's marks.
    pub(crate) fn marks(&self) -> (&[FoundMatch], Option<usize>) {
        (&self.matches, self.current)
    }
}

impl Shell {
    /// Whether Mod+F is a terminal's: a terminal pane or its find bar has
    /// the keyboard (the pane's own key handler opens its bar then, and the
    /// conversation's find stands down; coordinator, P3 question 3).
    pub(crate) fn find_goes_to_terminal(&self, window: &Window, cx: &gpui::App) -> bool {
        self.terminal_focused(window) || self.terminal_find_focused(window, cx)
    }

    /// Whether a terminal tab's find field has the keyboard.
    pub(crate) fn terminal_find_focused(&self, window: &Window, cx: &gpui::App) -> bool {
        self.areas.terminal_tabs.panes.values().any(|pane| {
            pane.find
                .as_ref()
                .is_some_and(|find| find.field.focus_handle(cx).is_focused(window))
        })
    }

    /// Mod+F in a tab: open its bar, or give the bar's field the keyboard.
    pub(super) fn terminal_find_open(
        &mut self,
        key: &PaneKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let placeholder = self.words.terminal_tabs().find_placeholder().to_owned();
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        if let Some(find) = &pane.find {
            find.field.focus(window, cx);
            return;
        }
        let field = TextField::with_source(
            &placeholder,
            Self::terminal_find_entered,
            Self::terminal_find_changed,
            window,
            cx,
        );
        field.set_accessible_label(&placeholder, cx);
        field.focus(window, cx);
        pane.find = Some(PaneFind {
            field,
            query: String::new(),
            finder: None,
            matches: Vec::new(),
            limited: false,
            current: None,
            keep: None,
            ticket: 0,
            task: None,
            searched: None,
            bottom: false,
        });
        cx.notify();
    }

    /// Escape or the close button: the marks go and the terminal has the
    /// keyboard again.
    pub(super) fn terminal_find_close(
        &mut self,
        key: &PaneKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        if pane.find.take().is_none() {
            return;
        }
        pane.marked();
        self.terminal_tab_focus(key, window, cx);
        self.terminal_tab_draw(key, cx);
        cx.notify();
    }

    /// Next (`forward`) or previous match, round the ends.
    pub(super) fn terminal_find_move(
        &mut self,
        key: &PaneKey,
        forward: bool,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(find) = self
            .areas
            .terminal_tabs
            .panes
            .get_mut(key)
            .and_then(|pane| pane.find.as_mut())
        else {
            return;
        };
        let count = find.matches.len();
        if count == 0 {
            return;
        }
        find.current = Some(match find.current {
            Some(at) if forward => next_of(at, count),
            Some(at) => previous_of(at, count),
            None if forward => 0,
            None => count.saturating_sub(1),
        });
        self.terminal_find_reveal(key, cx);
    }

    /// The pane whose find field `source` is.
    fn terminal_find_owner(&self, source: FieldId) -> Option<PaneKey> {
        self.areas
            .terminal_tabs
            .panes
            .iter()
            .find(|(_, pane)| pane.find.as_ref().is_some_and(|find| find.field.id() == source))
            .map(|(key, _)| key.clone())
    }

    fn terminal_find_entered(
        &mut self,
        source: FieldId,
        _text: String,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(key) = self.terminal_find_owner(source) {
            self.terminal_find_move(&key, true, cx);
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "the field's change callback hands the text over by value"
    )]
    pub(super) fn terminal_find_changed(
        &mut self,
        source: FieldId,
        text: String,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(key) = self.terminal_find_owner(source) else {
            return;
        };
        let query: String = text.chars().take(MAX_FIND_QUERY).collect();
        let Some(find) = self
            .areas
            .terminal_tabs
            .panes
            .get_mut(&key)
            .and_then(|pane| pane.find.as_mut())
        else {
            return;
        };
        if query == find.query {
            return;
        }
        find.query = query;
        find.keep = None;
        self.terminal_find_search(&key, cx);
    }

    /// New output while the bar is open: search again, keeping the current
    /// match on its text.
    pub(super) fn terminal_find_output(&mut self, key: &PaneKey, cx: &mut Context<'_, Self>) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        let Some(generation) = pane.model.as_ref().map(|model| model.grid().generation()) else {
            return;
        };
        let Some(find) = pane.find.as_mut() else {
            return;
        };
        if find.query.is_empty() || find.finder.is_some() || find.searched == Some(generation) {
            return;
        }
        find.keep = find
            .current
            .and_then(|at| find.matches.get(at))
            .map(|found| found.start);
        self.terminal_find_search(key, cx);
    }

    /// Start a search for the bar's query, dropping one running.
    fn terminal_find_search(&mut self, key: &PaneKey, cx: &mut Context<'_, Self>) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        let grid = pane.model.as_ref().map(|model| model.grid());
        let Some(find) = pane.find.as_mut() else {
            return;
        };
        find.ticket = find.ticket.wrapping_add(1);
        find.task = None;
        find.searched = grid.map(vessel_app_view::terminal::grid::Grid::generation);
        find.finder = grid.and_then(|grid| Finder::new(&find.query, grid));
        if find.finder.is_none() {
            // An empty query (or no grid yet): no marks.
            find.matches.clear();
            find.limited = false;
            find.current = None;
            pane.marked();
            self.terminal_tab_draw(key, cx);
            cx.notify();
            return;
        }
        let ticket = find.ticket;
        // The first slice at once: a usual history is searched before the
        // next frame, and only a long one goes on a slice a step.
        if self.terminal_find_step(key, ticket, cx) == Some(Stepped::Done) {
            return;
        }
        let stepped = key.clone();
        let Some(find) = self
            .areas
            .terminal_tabs
            .panes
            .get_mut(key)
            .and_then(|pane| pane.find.as_mut())
        else {
            return;
        };
        find.task = Some(cx.spawn(async move |this, cx| {
            loop {
                let Ok(Some(done)) =
                    this.update(cx, |shell, cx| shell.terminal_find_step(&stepped, ticket, cx))
                else {
                    return;
                };
                match done {
                    Stepped::Done => return,
                    Stepped::More => cx.background_executor().timer(Duration::ZERO).await,
                    Stepped::Away => cx.background_executor().timer(AWAY).await,
                }
            }
        }));
    }

    /// One slice of `ticket`'s search; `None` when it was dropped.
    fn terminal_find_step(
        &mut self,
        key: &PaneKey,
        ticket: u64,
        cx: &mut Context<'_, Self>,
    ) -> Option<Stepped> {
        let pane = self.areas.terminal_tabs.panes.get_mut(key)?;
        let find = pane.find.as_mut().filter(|find| find.ticket == ticket)?;
        let Some(grid) = pane.model.as_ref().map(|model| model.grid()) else {
            return Some(Stepped::Away);
        };
        let finder = find.finder.as_mut()?;
        if !finder.step(grid, ROWS_PER_STEP) {
            return Some(Stepped::More);
        }
        let finder = find.finder.take()?;
        find.limited = finder.limited();
        find.matches = finder.matches().to_vec();
        let kept = find
            .keep
            .take()
            .and_then(|at| find.matches.iter().position(|found| found.start == at));
        find.current = kept.or_else(|| {
            let scroll = pane.viewport.offset(grid);
            first_from(&find.matches, grid.text_at(scroll, 0, 0))
        });
        find.task = None;
        self.terminal_find_reveal(key, cx);
        Some(Stepped::Done)
    }

    /// Scroll the view to the current match when it is out of view, and
    /// redraw its marks.
    fn terminal_find_reveal(&mut self, key: &PaneKey, cx: &mut Context<'_, Self>) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        if let (Some(grid), Some(find)) = (pane.model.as_ref().map(|model| model.grid()), &pane.find)
            && let Some(found) = find.current.and_then(|at| find.matches.get(at))
        {
            let scroll = pane.viewport.offset(grid);
            if grid.view_row(found.start, scroll).is_none() {
                let wanted = grid.scroll_to_show(found.start);
                pane.viewport = vessel_app_view::terminal::Viewport::default();
                pane.viewport.earlier(wanted, grid);
            }
        }
        pane.marked();
        self.terminal_tab_draw(key, cx);
        cx.notify();
    }
}

/// What a step left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stepped {
    Done,
    More,
    Away,
}
