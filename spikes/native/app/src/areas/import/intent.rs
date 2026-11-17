//! Outgoing intentions bind to the rendered opening before any UI or RPC effect.
use super::Page;
use crate::shell::Shell;
use vessel_app_kit::ButtonState;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::devices::Ticket;
use vessel_app_view::history::Filter;
use vessel_proto::{HarnessRef, HarnessSessionId};

/// The opening a control was drawn for.
#[derive(Clone)]
pub(super) struct Capture {
    visit: Option<Ticket>,
}
impl Capture {
    pub(super) const fn of(shell: &Shell) -> Self {
        Self {
            visit: shell.areas.import.visit,
        }
    }
    pub(super) fn accepts(&self, shell: &Shell) -> bool {
        let area = &shell.areas.import;
        area.open && self.visit.is_some() && area.visit == self.visit
    }
}

#[derive(Clone)]
pub(super) enum Action {
    Close,
    /// The first page's host row.
    Host(String),
    /// The header's refresh icon.
    Refresh,
    /// The scope line's "Show all".
    ShowAll,
    /// The provider pill.
    FilterOpen,
    /// A row of the provider pill's list.
    Filter(Filter),
    /// A failed provider's Retry.
    Retry(HarnessRef),
    /// "Load more".
    More,
    /// A session row: import it.
    Row {
        harness: HarnessRef,
        id: HarnessSessionId,
    },
}

impl Action {
    /// Whether this action changes what the sheet is open for, so every
    /// control drawn before it must refuse.
    const fn moves(&self) -> bool {
        matches!(self, Self::Close | Self::Host(_) | Self::ShowAll)
    }
}

/// Paseo's disabled states: rows and Show all while an import runs; Refresh
/// and Load more while reading too (`isRefreshing`).
pub(super) fn state(action: &Action, shell: &Shell) -> ButtonState {
    let area = &shell.areas.import;
    let model = &area.model;
    let importing = model.importing().is_some();
    let ready = match action {
        Action::Close | Action::FilterOpen | Action::Filter(_) => true,
        Action::Host(_) => area.page == Page::Hosts,
        Action::Row { .. } | Action::ShowAll | Action::Retry(_) => {
            area.page == Page::Sessions && !importing
        }
        Action::Refresh | Action::More => {
            area.page == Page::Sessions && !importing && !model.loading()
        }
    };
    if ready {
        ButtonState::Enabled
    } else {
        ButtonState::Disabled
    }
}

pub(super) fn listener(
    shell: &Shell,
    action: Action,
) -> impl Fn(&mut Shell, &mut Window, &mut Context<'_, Shell>) + Clone + 'static {
    let capture = Capture::of(shell);
    move |shell, window, cx| {
        if !capture.accepts(shell) || state(&action, shell) == ButtonState::Disabled {
            return;
        }
        if action.moves() && !shell.areas.import.advance_visit() {
            cx.notify();
            return;
        }
        match &action {
            Action::Close => shell.close_import(window, cx),
            Action::Host(name) => shell.history_host(name, None, window, cx),
            Action::Refresh => shell.history_refresh(),
            Action::ShowAll => shell.history_show_all(),
            Action::FilterOpen => shell.history_filter_open(window, cx),
            Action::Filter(filter) => {
                shell.areas.import.filter.close_and_restore(window, cx);
                shell.history_filter(filter.clone());
            }
            Action::Retry(harness) => shell.history_retry(harness),
            Action::More => shell.history_more(),
            Action::Row { harness, id } => shell.history_row(harness, id),
        }
        cx.notify();
    }
}
