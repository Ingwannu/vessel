//! A terminal tab's stream: following it, its records parsed off the UI
//! thread (history silently, live output answering queries per ADR 0073's
//! `query_duty`), the sender's one send in flight, and the link's answers.

use gpui::AppContext as _;
use vessel_app_kit::ui::Context;
use vessel_app_link::Command;
use vessel_app_link::terminals::{Answer, Ask, Reply, Request};
use vessel_app_view::terminal::{Focus, Others, QueryDuty, TerminalModel, query_duty};
use vessel_app_view::terminal_tabs::{Failure, Trouble};
use vessel_proto::{EventRecord, TerminalId};

use super::{PaneKey, Stage};
use crate::areas::terminal::replay::{refit, replay_all};
use crate::shell::Shell;

/// Parse `bytes` as live output, collecting the replies to its queries.
fn feed_all(model: &mut TerminalModel, bytes: &[u8]) -> Vec<u8> {
    let mut replies = Vec::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        let fed = model.feed(rest);
        if fed.consumed == 0 {
            break;
        }
        replies.extend_from_slice(&fed.replies);
        rest = rest.get(fed.consumed..).unwrap_or_default();
    }
    replies
}

impl Shell {
    /// Ask the link to follow `key`'s terminal from where its log stands.
    pub(super) fn terminal_tab_follow(&mut self, key: &PaneKey) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        pane.stage = Stage::Attaching;
        // An attach claims the size once it is live (Paseo's stream
        // controller claims on attach).
        pane.claimed = false;
        let from = pane.log.next();
        self.command(Command::Terminals(Ask {
            machine: key.machine.clone(),
            request: Request::Follow {
                terminal: key.terminal,
                from,
            },
        }));
    }

    /// A fleet read: the shown workspace's shells are listed again, and
    /// tabs that lost their machine follow again.
    pub(crate) fn terminal_tabs_on_fleet(&mut self) {
        self.terminal_tabs_poll();
        self.terminal_tabs_rejoin();
    }

    /// Tabs that lost their machine follow again once it reads online
    /// (Paseo's stream controller attaches again on reconnect), from where
    /// their log stands, and claim their size again.
    pub(crate) fn terminal_tabs_rejoin(&mut self) {
        let back: Vec<PaneKey> = self
            .areas
            .terminal_tabs
            .panes
            .iter()
            .filter(|(_, pane)| pane.stage == Stage::Offline)
            .map(|(key, _)| key.clone())
            .filter(|key| self.machine_online(&key.machine))
            .collect();
        for key in back {
            if let Some(pane) = self.areas.terminal_tabs.panes.get_mut(&key) {
                pane.sent_size = None;
            }
            self.terminal_tab_follow(&key);
        }
    }

    /// The next send of `key`'s sender, when none is in flight.
    pub(super) fn terminal_tab_pump(&mut self, key: &PaneKey) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        if pane.stage != Stage::Live {
            return;
        }
        if let Some(outgoing) = pane.sender.take() {
            self.command(Command::Terminals(Ask {
                machine: key.machine.clone(),
                request: Request::Send {
                    terminal: key.terminal,
                    outgoing,
                },
            }));
        }
    }

    /// A link answer about terminals.
    pub(crate) fn terminal_tabs_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        let Answer { machine, reply } = answer;
        let key = |terminal: TerminalId| PaneKey {
            machine: machine.clone(),
            terminal,
        };
        match reply {
            Reply::Created { place, result } => {
                self.terminal_tab_created(&machine, &place, result, cx);
            }
            Reply::Listed {
                place,
                result,
                waits,
            } => {
                self.terminal_tabs_listed(&machine, &place, result.as_ref().ok(), waits);
            }
            Reply::Following(info) => {
                let key = key(info.id);
                if let Some(pane) = self.areas.terminal_tabs.panes.get_mut(&key) {
                    pane.live_from = info.next_seq;
                    pane.info = Some(*info);
                    self.terminal_tab_settle(&key, cx);
                }
            }
            Reply::Record { terminal, record } => {
                self.terminal_tab_record(&key(terminal), record, cx);
            }
            Reply::Ended { terminal } => {
                let key = key(terminal);
                self.terminal_tab_trouble(&key, Trouble::Exited, cx);
                // An exited terminal leaves the list, and its tab with it.
                self.terminal_tabs_poll();
            }
            Reply::Lost { terminal, failure } => {
                let key = key(terminal);
                // A dropped connection is the host's, not the terminal's:
                // the tab follows again once the machine is back
                // (`terminal_tabs_rejoin`).
                if failure != Failure::Offline && self.machine_online(&key.machine) {
                    self.terminal_tab_trouble(&key, failure.trouble(), cx);
                } else if let Some(pane) = self.areas.terminal_tabs.panes.get_mut(&key) {
                    pane.stage = Stage::Offline;
                    cx.notify();
                }
            }
            Reply::Sent { terminal, .. } => {
                let key = key(terminal);
                if let Some(pane) = self.areas.terminal_tabs.panes.get_mut(&key) {
                    pane.sender.answered();
                }
                self.terminal_tab_pump(&key);
            }
            Reply::Closed { result, .. } => {
                // Paseo only refreshes its list when a kill fails.
                if result.is_err() {
                    tracing::warn!("a terminal could not be closed on its machine");
                }
                self.terminal_tabs_poll();
            }
        }
    }

    fn terminal_tab_trouble(
        &mut self,
        key: &PaneKey,
        trouble: Trouble,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) {
            pane.stage = Stage::Trouble(trouble);
            pane.sender.close();
            cx.notify();
        }
    }

    /// One followed record: history or live bytes for the parser.
    fn terminal_tab_record(
        &mut self,
        key: &PaneKey,
        record: EventRecord,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        let history = record.seq < pane.live_from;
        let records = [record];
        for bytes in pane.log.apply(&records) {
            if history {
                pane.history.extend_from_slice(bytes);
            } else {
                pane.live.extend_from_slice(bytes);
            }
        }
        self.terminal_tab_settle(key, cx);
        self.terminal_tab_apply(key, cx);
    }

    /// Attaching ends once every history record arrived.
    fn terminal_tab_settle(&mut self, key: &PaneKey, cx: &mut Context<'_, Self>) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        if pane.stage == Stage::Attaching
            && pane.info.is_some()
            && pane.log.next() >= pane.live_from
        {
            pane.stage = Stage::Live;
            cx.notify();
            self.terminal_tab_claim(key);
            self.terminal_tab_pump(key);
        }
    }

    /// Hand the pending bytes to the background executor, when the model
    /// is not already there; zero parsing on the UI thread.
    pub(super) fn terminal_tab_apply(&mut self, key: &PaneKey, cx: &mut Context<'_, Self>) {
        let focused = self.terminal_tab_has_focus(key);
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        if pane.history.is_empty() && pane.live.is_empty() {
            self.terminal_tab_draw(key, cx);
            return;
        }
        let Some(mut model) = pane.model.take() else {
            return;
        };
        let history = std::mem::take(&mut pane.history);
        let live = std::mem::take(&mut pane.live);
        let duty = query_duty(pane.origin, focused, pane.typed, Others::NoneQualify);
        let fit = pane.fit;
        let ticket = std::rc::Rc::clone(&pane.ticket);
        let parse = cx.background_spawn(async move {
            refit(&mut model, fit);
            replay_all(&mut model, &history);
            let replies = match duty {
                QueryDuty::Answer => feed_all(&mut model, &live),
                QueryDuty::Silent => {
                    replay_all(&mut model, &live);
                    Vec::new()
                }
            };
            (model, replies)
        });
        let key = key.clone();
        cx.spawn(async move |this, cx| {
            let (model, replies) = parse.await;
            drop(this.update(cx, |shell, cx| {
                shell.terminal_tab_parsed(&key, &ticket, model, &replies, cx);
            }));
        })
        .detach();
    }

    /// A parse came back: the model returns, its replies go out in order
    /// with the keys, and anything that arrived meanwhile is parsed next.
    fn terminal_tab_parsed(
        &mut self,
        key: &PaneKey,
        ticket: &std::rc::Rc<()>,
        mut model: TerminalModel,
        replies: &[u8],
        cx: &mut Context<'_, Self>,
    ) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        // A pane opened again under the same key parses its own model.
        if !std::rc::Rc::ptr_eq(&pane.ticket, ticket) {
            return;
        }
        refit(&mut model, pane.fit);
        pane.model = Some(model);
        if let Ok(replies) = std::str::from_utf8(replies)
            && !replies.is_empty()
        {
            let _queued = pane.sender.keys(replies);
        }
        self.terminal_tab_pump(key);
        self.terminal_tab_draw(key, cx);
        self.terminal_tab_apply(key, cx);
    }

    /// Whether `key`'s pane has the keyboard, as `query_duty` reads it.
    fn terminal_tab_has_focus(&self, key: &PaneKey) -> Focus {
        if self.areas.terminal_tabs.focused.as_ref() == Some(key) {
            Focus::Held
        } else {
            Focus::Lost
        }
    }
}
