//! Outgoing intentions bind to the rendered visit before any UI or RPC effect.
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::{
    devices::Ticket,
    history::{Scope, Selection},
};
use vessel_proto::HarnessSessionId;

#[derive(Clone)]
pub(super) struct Capture {
    visit: Option<Ticket>,
    scope: Option<Scope>,
}
impl Capture {
    pub(super) fn of(shell: &Shell) -> Self {
        Self {
            visit: shell.areas.import.visit,
            scope: shell.areas.import.model.scope().cloned(),
        }
    }
    pub(super) fn accepts(&self, shell: &Shell) -> bool {
        let area = &shell.areas.import;
        area.open
            && self.visit.is_some()
            && area.visit == self.visit
            && area.model.scope() == self.scope.as_ref()
    }
}
#[derive(Clone)]
pub(super) enum Action {
    Refresh,
    Cancel,
    Import,
    More,
    ShowAll,
    Row {
        id: HarnessSessionId,
        selection: Selection,
    },
}

pub(super) fn listener(
    shell: &Shell,
    action: Action,
) -> impl Fn(&mut Shell, &mut Window, &mut Context<'_, Shell>) + Clone + 'static {
    let capture = Capture::of(shell);
    move |shell, window, cx| {
        if !capture.accepts(shell)
            || super::draw::state(&action, &shell.areas.import.model)
                == vessel_app_kit::ButtonState::Disabled
        {
            return;
        }
        if let Action::Row { id, selection } = &action
            && shell.areas.import.model.selection(id) != *selection
        {
            return;
        }
        // Invalidate every previously rendered intention before an accepted effect.
        if !shell.areas.import.advance_visit() {
            return;
        }
        match &action {
            Action::Refresh => {
                shell.history_profiles();
                shell.history_refresh();
            }
            Action::Cancel => shell.close_import(window, cx),
            Action::Import => shell.history_import(),
            Action::More => shell.history_more(),
            Action::ShowAll => shell.history_show_all(),
            Action::Row { id, .. } => shell.history_row(id, window, cx),
        }
        cx.notify();
    }
}
