//! Paseo's import sheet (`import-session-sheet.tsx`, `use-import-session.tsx`
//! and the host chooser): the rules live in view's `Sheet`, the calls in link.
//!
//! [Decision Log] TRACKER 777 (2026-11-17)
//! - Purpose: the coordinator's rulings: Paseo's behaviour, not W2's. A row
//!   press imports that one session, closes the sheet and opens it; every
//!   available agent of the host is read in parallel and merged; a provider
//!   pill filters the merged list on the client.
//! - Before: W2's three stacked pickers (machine, agent, account) over one
//!   scope, checkbox rows, an Import selected / Cancel footer and a virtual
//!   list fixed at 30 % of the window.
//! - Alternatives: keep the pickers beside a provider pill (two ways to choose
//!   a provider, ruled out); keep the virtual list inside the dialog body (a
//!   second scroller under the compact sheet, whose drag hands off to the
//!   body's own scroll).
//! - Choice: the host comes from the caller (the new-session page's machine);
//!   without one the sheet's first page is Paseo's "Import from host", which
//!   skips itself when one host is connected, as `useHostChooser` does. Rows
//!   are drawn in the dialog body, which scrolls and hands its scroll to the
//!   sheet's drag. A finished import is opened on the next frame with a
//!   window (`settle_import`), as `settle_sessions` opens a located session.
//! - Cost: rows are not virtualised. The per-provider limits (at most 200)
//!   bound what is drawn.

mod actions;
mod draw;
mod filter;
mod hosts;
mod intent;
mod rows;
#[cfg(test)]
mod tests;
pub(crate) use draw::{entry, render};

use crate::areas::session::Open;
use crate::shell::Shell;
use gpui::FocusHandle;
use std::rc::Rc;
use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::field::SearchField;
use vessel_app_kit::tooltip::Place;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_kit::{ComboboxState, Tooltips};
use vessel_app_view::devices::{Ticket, Tickets};
use vessel_app_view::history::{Problem, Sheet};
use vessel_app_view::matcher::Query;

/// The sheet's two pages (Paseo's host chooser, then the sheet itself).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Page {
    /// "Import from host": no host was in context.
    Hosts,
    /// The sessions of the chosen host.
    Sessions,
}

/// The sheet's controls and state, independent from a route.
pub(crate) struct ImportArea {
    pub(super) open: bool,
    pub(super) page: Page,
    pub(super) model: Sheet,
    pub(super) search: SearchField,
    pub(super) query: Query,
    pub(super) dialog: DialogState,
    pub(super) filter: ComboboxState,
    pub(super) filter_anchor: vessel_app_kit::select::Anchor,
    pub(super) tips: Tooltips,
    pub(super) refresh_place: Place,
    pub(super) tickets: Tickets,
    pub(super) failure: Option<Problem>,
    /// The imported session, opened on the next frame with a window.
    pub(super) opened: Option<Open>,
    pub(super) prior: Option<FocusHandle>,
    pub(super) visit: Option<Ticket>,
    /// Counts what was typed, so only the last search is sent (Paseo's
    /// 200 ms debounce, `SEARCH_SETTLE`).
    pub(super) typed: u64,
}

/// How long the search rests before a ranked host is asked (Paseo's
/// `SEARCH_DEBOUNCE_MS`).
pub(super) const SEARCH_SETTLE: std::time::Duration = std::time::Duration::from_millis(200);

impl ImportArea {
    /// Whether the sheet is open (another area's test asks).
    #[cfg(test)]
    pub(crate) const fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            open: false,
            page: Page::Sessions,
            model: Sheet::default(),
            search: SearchField::new(
                "",
                |shell: &mut Shell, text, cx| {
                    shell.areas.import.query = Query::new(&text);
                    shell.history_search(&text, cx);
                    cx.notify();
                },
                window,
                cx,
            ),
            query: Query::new(""),
            dialog: DialogState::new(cx),
            filter: ComboboxState::new(window, cx),
            filter_anchor: Rc::default(),
            tips: Tooltips::new(),
            refresh_place: Rc::default(),
            tickets: Tickets::default(),
            failure: None,
            opened: None,
            prior: None,
            visit: None,
            typed: 0,
        }
    }

    /// One persistent ticket source fences every rendered intention to the
    /// opening (and page and host) it was drawn for, including A -> B -> A.
    fn advance_visit(&mut self) -> bool {
        self.filter.close();
        if let Ok(ticket) = self.tickets.next_ticket() {
            self.visit = Some(ticket);
            true
        } else {
            self.visit = None;
            self.failure = Some(Problem::Exhausted);
            false
        }
    }
}

/// The search's placeholder follows the page and the language.
pub(crate) fn refresh_words(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let words = shell.words.history();
    let placeholder = match shell.areas.import.page {
        Page::Hosts => words.search_hosts(),
        Page::Sessions => words.search(),
    };
    shell
        .areas
        .import
        .search
        .set_placeholder(placeholder, window, cx);
}
