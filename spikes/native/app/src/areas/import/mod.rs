//! Own-history import sheet; rules live in view, existing calls in link.

mod actions;
mod draw;
mod intent;
mod pickers;
mod rows;
#[cfg(test)]
mod tests;
pub(crate) use draw::{entry, render};

use crate::shell::Shell;
use gpui::FocusHandle;
use std::rc::Rc;
use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_kit::{ComboboxState, TextField, VirtualList};
use vessel_app_view::devices::{Ticket, Tickets};
use vessel_app_view::history::{History, Problem, Scope};
use vessel_app_view::matcher::Query;
use vessel_proto::{HarnessSessionId, ProfileInfo};

/// A picker never routes on a string label supplied by a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Picker {
    Machine,
    Agent,
    Account,
}

/// The sheet's controls and prepared projection, independent from a route.
pub(crate) struct ImportArea {
    pub(super) open: bool,
    pub(super) model: History,
    pub(super) search: TextField,
    pub(super) query: Query,
    pub(super) visible: Vec<HarnessSessionId>,
    pub(super) list: VirtualList,
    pub(super) dialog: DialogState,
    pub(super) picker: ComboboxState,
    pub(super) choosing: Option<Picker>,
    pub(super) machine_anchor: vessel_app_kit::select::Anchor,
    pub(super) agent_anchor: vessel_app_kit::select::Anchor,
    pub(super) account_anchor: vessel_app_kit::select::Anchor,
    pub(super) profiles: Result<Vec<ProfileInfo>, Problem>,
    pub(super) profile_request: Option<(Scope, Ticket)>,
    pub(super) tickets: Tickets,
    pub(super) failure: Option<Problem>,
    pub(super) prior: Option<FocusHandle>,
    pub(super) visit: Option<Ticket>,
}

impl ImportArea {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            open: false,
            model: History::default(),
            search: TextField::with_change(
                "",
                |_, _, _, _| {},
                |shell: &mut Shell, text, cx| {
                    shell.areas.import.query = Query::new(&text);
                    shell.areas.import.reproject();
                    cx.notify();
                },
                window,
                cx,
            ),
            query: Query::new(""),
            visible: Vec::new(),
            list: VirtualList::new(false),
            dialog: DialogState::new(cx),
            picker: ComboboxState::new(window, cx),
            choosing: None,
            machine_anchor: Rc::default(),
            agent_anchor: Rc::default(),
            account_anchor: Rc::default(),
            profiles: Ok(Vec::new()),
            profile_request: None,
            tickets: Tickets::default(),
            failure: None,
            prior: None,
            visit: None,
        }
    }

    /// One persistent ticket source prevents close/reopen and scope ABA.
    fn advance_visit(&mut self) -> bool {
        match self.tickets.next_ticket() {
            Ok(ticket) => {
                self.visit = Some(ticket);
                true
            }
            Err(_) => {
                self.visit = None;
                self.failure = Some(Problem::Exhausted);
                false
            }
        }
    }

    /// All scope changes share the visit fence, including A -> B -> A.
    fn choose(&mut self, scope: Scope) -> bool {
        if self.model.importing() {
            return false;
        }
        if self.model.scope() != Some(&scope) && !self.advance_visit() {
            return false;
        }
        self.model.choose(scope);
        true
    }

    fn reproject(&mut self) {
        let visible: Vec<_> = self
            .model
            .filtered(&self.query)
            .into_iter()
            .map(|entry| entry.id.clone())
            .collect();
        // Appending an older page must not put the person back at the top.
        if visible.starts_with(&self.visible) {
            let old = self.visible.len();
            self.list
                .splice(old..old, visible.len().saturating_sub(old));
            self.list.changed(0..old);
        } else {
            self.list.reset(visible.len());
        }
        self.visible = visible;
    }
}

/// Refresh retained placeholders without changing search text or selection.
pub(crate) fn refresh_words(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell
        .areas
        .import
        .search
        .set_placeholder(shell.words.history().search(), window, cx);
}
