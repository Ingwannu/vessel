//! User actions retain the scope before sending any request.

use super::Picker;
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_view::devices::MachineKey;
use vessel_app_view::history::{Ask, Problem, Reply, Scope, Selection};
use vessel_proto::HarnessSessionId;

impl Shell {
    pub(crate) fn open_import(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.import;
        if !area.advance_visit() {
            return;
        }
        area.open = true;
        area.prior = window.focused(cx);
        area.failure = None;
        area.dialog.focus(window, cx);
        super::refresh_words(self, window, cx);
        if !self.areas.import.model.importing() {
            self.areas.import.search.clear(window, cx);
            self.areas.import.query = vessel_app_view::matcher::Query::new("");
            let machine = self
                .areas
                .draft
                .machine
                .as_ref()
                .and_then(|name| self.fleet.online().find(|m| &m.name == name))
                .or_else(|| self.fleet.online().next());
            if let Some(machine) = machine {
                let harness = self
                    .areas
                    .draft
                    .agent
                    .as_ref()
                    .filter(|h| {
                        machine
                            .agents
                            .iter()
                            .any(|a| &a.harness == *h && a.available())
                    })
                    .cloned()
                    .or_else(|| {
                        machine
                            .agents
                            .iter()
                            .find(|a| a.available())
                            .map(|a| a.harness.clone())
                    });
                if let Some(harness) = harness
                    && let Some(context) = machine.connection_context()
                {
                    let cwd = (!self.areas.draft.folder.is_empty())
                        .then(|| std::path::PathBuf::from(&self.areas.draft.folder));
                    self.areas.import.choose(Scope {
                        context,
                        machine: MachineKey::from_book(&machine.name),
                        harness,
                        profile: None,
                        cwd,
                    });
                    self.areas.import.profiles = Ok(Vec::new());
                    self.history_profiles();
                    self.history_refresh();
                }
            }
        }
        self.areas.import.reproject();
        self.areas.import.search.focus(window, cx);
        cx.notify();
    }

    pub(super) fn close_import(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.import;
        area.open = false;
        area.visit = None;
        area.picker.close();
        area.choosing = None;
        area.profile_request = None;
        if !area.model.importing() {
            area.model.close();
        }
        if let Some(prior) = area.prior.take() {
            prior.focus(window, cx);
        }
        cx.notify();
    }

    pub(super) fn history_refresh(&mut self) {
        let ask = self.areas.import.model.refresh();
        self.history_submit(ask);
    }

    pub(super) fn history_more(&mut self) {
        let ask = self.areas.import.model.more();
        self.history_submit(ask);
    }

    pub(super) fn history_import(&mut self) {
        let ask = self.areas.import.model.import();
        self.history_submit(ask);
    }

    fn history_submit(&mut self, ask: Result<Option<Ask>, Problem>) {
        match ask {
            Ok(Some(ask)) => {
                self.areas.import.failure = None;
                self.command(Command::History(ask));
            }
            Ok(None) => {}
            Err(problem) => self.areas.import.failure = Some(problem),
        }
    }

    pub(super) fn history_profiles(&mut self) {
        let Some(scope) = self.areas.import.model.scope().cloned() else {
            return;
        };
        match self.areas.import.tickets.next_ticket() {
            Ok(ticket) => {
                self.areas.import.profile_request = Some((scope.clone(), ticket));
                self.command(Command::History(Ask::Profiles { scope, ticket }));
            }
            Err(_) => self.areas.import.failure = Some(Problem::Exhausted),
        }
    }

    pub(crate) fn history_answer(
        &mut self,
        answer: vessel_app_link::history::Answer,
        cx: &mut Context<'_, Self>,
    ) {
        self.history_apply_reply(answer.into_reply(), cx);
    }

    pub(super) fn history_apply_reply(&mut self, reply: Reply, cx: &mut Context<'_, Self>) {
        if let Reply::Profiles {
            scope,
            ticket,
            result,
        } = reply
        {
            if self.areas.import.profile_request.as_ref() == Some(&(scope, ticket)) {
                self.areas.import.profiles = result;
                self.areas.import.profile_request = None;
                self.areas.import.advance_visit();
                cx.notify();
            }
            return;
        }
        if self.areas.import.model.answer(reply) {
            self.areas.import.advance_visit();
            self.areas.import.reproject();
        }
        // Receipt installation never navigates or reloads an alias. A later
        // guarded resident opener must own that effect through its final admission.
        cx.notify();
    }

    pub(super) fn history_row(
        &mut self,
        id: &HarnessSessionId,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.import.model.importing() {
            return;
        }
        match self.areas.import.model.selection(id) {
            Selection::Existing(_) => {
                let ask = self.areas.import.model.activate(id);
                self.history_submit(ask);
                cx.notify();
            }
            Selection::Available | Selection::Selected => {
                self.areas.import.model.toggle(id);
                self.areas.import.advance_visit();
                cx.notify();
            }
        }
    }

    pub(super) fn history_show_all(&mut self) {
        if self.areas.import.model.importing() {
            return;
        }
        if let Some(mut scope) = self.areas.import.model.scope().cloned() {
            scope.cwd = None;
            self.areas.import.choose(scope);
            self.areas.import.reproject();
            self.history_refresh();
        }
    }

    pub(super) fn history_picker(
        &mut self,
        which: Picker,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.import.model.importing() {
            return;
        }
        let anchor = match which {
            Picker::Machine => &self.areas.import.machine_anchor,
            Picker::Agent => &self.areas.import.agent_anchor,
            Picker::Account => &self.areas.import.account_anchor,
        };
        self.areas.import.picker.set_anchor(anchor.get());
        self.areas.import.choosing = Some(which);
        self.areas.import.picker.open(None, window, cx);
        cx.notify();
    }
}
