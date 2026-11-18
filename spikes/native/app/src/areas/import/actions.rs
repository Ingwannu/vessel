//! Opening, reading and importing; every request leaves through `history_submit`.

use super::Page;
use crate::areas::session::Open;
use crate::shell::Shell;
use std::path::PathBuf;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_view::Card;
use vessel_app_view::devices::MachineKey;
use vessel_app_view::history::{Ask, Filter, Opening, Problem, Reply, Scope};
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

    /// Paseo's workspace header Import session (TRACKER 789, the compact
    /// header's "…"): the open session's host and folder, so the sheet
    /// reads "This workspace" with "Show all".
    pub(crate) fn open_import_for_workspace(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let host = self
            .areas
            .session
            .open
            .as_ref()
            .map(|open| open.machine.clone());
        let cwd = self
            .areas
            .conversation
            .info
            .as_deref()
            .and_then(|info| info.spec.cwd.clone());
        self.open_import_for(host, cwd, window, cx);
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
        let Some((context, providers, ranked)) = self
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
                // ADR 0109: the node lists, searches and previews itself.
                let ranked = machine.protocol.is_some_and(|protocol| {
                    vessel_proto::history::RANKED_HISTORY_PROTOCOL
                        .is_some_and(|since| protocol >= since)
                });
                machine
                    .connection_context()
                    .map(|context| (context, providers, ranked))
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
            ranked,
        };
        let area = &mut self.areas.import;
        area.page = Page::Sessions;
        let asks = area.model.open(opening, providers);
        self.history_submit(asks);
        self.history_reset_search(window, cx);
    }

    /// Paseo resets the search per opening and per page.
    fn history_reset_search(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        // A search still resting belongs to the last opening: drop it.
        self.areas.import.typed = self.areas.import.typed.wrapping_add(1);
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

    /// The search changed: once it rests ([`SEARCH_SETTLE`](super::SEARCH_SETTLE)),
    /// a ranked host is asked again with it; an older one is filtered here.
    pub(super) fn history_search(&mut self, text: &str, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.import;
        area.typed = area.typed.wrapping_add(1);
        if !area.model.ranked() {
            return;
        }
        let typed = area.typed;
        let text = text.to_owned();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(super::SEARCH_SETTLE).await;
            let _gone = this.update(cx, |shell, cx| {
                if shell.areas.import.typed == typed && shell.areas.import.open {
                    let asks = shell.areas.import.model.search(&text);
                    shell.history_submit(asks);
                    cx.notify();
                }
            });
        })
        .detach();
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
        // A superseded reply changes nothing shown, yet frees a reading slot
        // for a read that was waiting.
        let follows = !answer.follow.is_empty();
        self.history_submit(Ok(answer.follow));
        if !answer.applied {
            if follows {
                cx.notify();
            }
            return;
        }
        if let Some((scope, info)) = self.areas.import.model.take_imported() {
            self.history_imported(&scope, &info);
        }
        cx.notify();
    }

    /// Paseo's `onImported`: open the new session where it was imported.
    /// Host and node are the import's own scope: neither the sheet's current
    /// opening (it may have been opened again on another host since) nor a
    /// later lookup of the machine's name (which may have been rebound).
    fn history_imported(&mut self, scope: &Scope, info: &SessionInfo) {
        let area = &mut self.areas.import;
        if !area.open {
            return;
        }
        let machine = scope.machine.as_str().to_owned();
        let Ok(host) = Name::parse(&machine) else {
            return;
        };
        let mut open = Open::of(&Card::from_session(&machine, info));
        open.binding = Some(vessel_app_link::sessions::Binding {
            host,
            address: SessionAddress {
                node: scope.context.node(),
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
