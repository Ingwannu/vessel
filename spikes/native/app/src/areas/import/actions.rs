//! Opening, reading and importing; every request leaves through `history_submit`.

use super::Page;
use crate::areas::session::Open;
use crate::shell::Shell;
use std::path::PathBuf;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_view::Card;
use vessel_app_view::devices::MachineKey;
use vessel_app_view::history::{Ask, Filter, Opening, Problem, Reply};
use vessel_app_view::machine_book::Name;
use vessel_app_view::matcher::Query;
use vessel_app_view::sessions::SessionAddress;
use vessel_proto::{HarnessRef, HarnessSessionId, SessionInfo};

impl Shell {
    /// Paseo's `useImportSession({ serverId }).open` on the new-workspace
    /// screen: the page's machine is the host, and no folder is passed, so
    /// the sheet lists the whole host ("Sessions on {host}").
    pub(crate) fn open_import(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let host = self
            .areas
            .draft
            .machine
            .clone()
            .filter(|name| self.fleet.online().any(|machine| &machine.name == name));
        self.open_import_for(host, None, window, cx);
    }

    /// Open for `host` (the chooser first without one) and, from a
    /// workspace, its folder (`cwd`, "This workspace" with "Show all").
    pub(super) fn open_import_for(
        &mut self,
        host: Option<String>,
        cwd: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.import;
        if !area.advance_visit() {
            cx.notify();
            return;
        }
        area.open = true;
        area.prior = window.focused(cx);
        area.failure = None;
        area.opened = None;
        area.dialog.focus(window, cx);
        // `useHostChooser`: a single connected host needs no choice.
        let host = host.or_else(|| {
            let mut online = self.fleet.online();
            match (online.next(), online.next()) {
                (Some(only), None) => Some(only.name.clone()),
                (Some(_), Some(_)) | (None, Some(_) | None) => None,
            }
        });
        if let Some(name) = host {
            self.history_host(&name, cwd, window, cx);
        } else {
            self.areas.import.model.close();
            self.areas.import.page = Page::Hosts;
            self.history_reset_search(window, cx);
            self.areas.import.search.focus(window, cx);
        }
        cx.notify();
    }

    /// Read every available agent of `name`, in parallel (Paseo's
    /// `providersToFetch`); the default account of each.
    pub(super) fn history_host(
        &mut self,
        name: &str,
        cwd: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some((context, providers)) = self
            .fleet
            .online()
            .find(|machine| machine.name == name)
            .and_then(|machine| {
                let providers: Vec<HarnessRef> = machine
                    .agents
                    .iter()
                    .filter(|agent| agent.available())
                    .map(|agent| agent.harness.clone())
                    .collect();
                machine
                    .connection_context()
                    .map(|context| (context, providers))
            })
        else {
            // Gone or offline since it was drawn: choose again.
            self.areas.import.model.close();
            self.areas.import.page = Page::Hosts;
            self.history_reset_search(window, cx);
            return;
        };
        let opening = Opening {
            context,
            machine: MachineKey::from_book(name),
            cwd,
        };
        let area = &mut self.areas.import;
        area.page = Page::Sessions;
        let asks = area.model.open(opening, providers);
        self.history_submit(asks);
        self.history_reset_search(window, cx);
    }

    /// Paseo resets the search per opening and per page.
    fn history_reset_search(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.import.search.clear(window, cx);
        self.areas.import.query = Query::new("");
        super::refresh_words(self, window, cx);
    }

    pub(super) fn close_import(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.import;
        area.filter.close();
        area.open = false;
        area.visit = None;
        area.page = Page::Sessions;
        // An import already sent keeps its receipt; it is not opened.
        area.model.close();
        if let Some(prior) = area.prior.take() {
            prior.focus(window, cx);
        }
        cx.notify();
    }

    pub(super) fn history_refresh(&mut self) {
        let asks = self.areas.import.model.refresh();
        self.history_submit(asks);
    }

    pub(super) fn history_more(&mut self) {
        let asks = self.areas.import.model.more();
        self.history_submit(asks);
    }

    pub(super) fn history_retry(&mut self, harness: &HarnessRef) {
        let asks = self.areas.import.model.retry(harness);
        self.history_submit(asks);
    }

    pub(super) fn history_show_all(&mut self) {
        let asks = self.areas.import.model.show_all();
        self.history_submit(asks);
    }

    pub(super) fn history_filter(&mut self, filter: Filter) {
        self.areas.import.model.set_filter(filter);
    }

    /// Paseo's row press: import that one session.
    pub(super) fn history_row(&mut self, harness: &HarnessRef, id: &HarnessSessionId) {
        let ask = self.areas.import.model.import(harness, id);
        self.history_submit(ask.map(|ask| ask.into_iter().collect()));
    }

    fn history_submit(&mut self, asks: Result<Vec<Ask>, Problem>) {
        match asks {
            Ok(asks) => {
                for ask in asks {
                    self.command(Command::History(ask));
                }
            }
            Err(problem) => self.areas.import.failure = Some(problem),
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
        let answer = self.areas.import.model.answer(reply);
        if !answer.applied {
            return;
        }
        self.history_submit(Ok(answer.follow));
        if let Some(info) = self.areas.import.model.take_imported() {
            self.history_imported(&info);
        }
        cx.notify();
    }

    /// Paseo's `onImported`: open the new session where it was imported.
    /// The node is the one the import went to, never a later lookup of the
    /// machine's name, which may have been rebound meanwhile.
    fn history_imported(&mut self, info: &SessionInfo) {
        let area = &mut self.areas.import;
        let Some(opening) = area.model.opening().filter(|_| area.open) else {
            return;
        };
        let machine = opening.machine.as_str().to_owned();
        let Ok(host) = Name::parse(&machine) else {
            return;
        };
        let mut open = Open::of(&Card::from_session(&machine, info));
        open.binding = Some(vessel_app_link::sessions::Binding {
            host,
            address: SessionAddress {
                node: opening.context.node(),
                session: info.id,
            },
        });
        area.opened = Some(open);
    }

    /// The imported session's opening needs a window: close the sheet, then
    /// open the session (its composer takes the focus the sheet returns).
    pub(crate) fn settle_import(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(open) = self.areas.import.opened.take() {
            self.close_import(window, cx);
            self.open_session(open, window, cx);
        }
    }
}
