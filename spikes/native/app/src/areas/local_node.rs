//! Per-window observations contain no child handle or process lifetime authority.

mod render;
pub(crate) use render::{dialog, section};

use crate::shell::Shell;
use gpui::{Context, Window};
use vessel_app_link::{
    Command,
    local_node::{
        Action, Snapshot,
        registration::{Answer, Ask},
    },
};
use vessel_app_view::devices::{Ticket, Tickets};
use vessel_app_view::local_node::{AfterQuit, LocalTarget, Management, Status};

pub(crate) struct LocalNodeArea {
    pub(crate) snapshot: Option<Snapshot>,
    target: Option<LocalTarget>,
    tickets: Tickets,
    pending: Option<Ticket>,
    pub(crate) quit_refused: bool,
    logs: Logs,
    logs_dialog: vessel_app_kit::dialog::DialogState,
}
impl LocalNodeArea {
    pub(crate) fn new(cx: &mut gpui::App) -> Self {
        Self {
            snapshot: None,
            target: None,
            tickets: Tickets::default(),
            pending: None,
            quit_refused: false,
            logs: Logs::Closed,
            logs_dialog: vessel_app_kit::dialog::DialogState::new(cx),
        }
    }
}

enum Logs {
    Closed,
    Loading(Ticket),
    Ready(vessel_app_link::local_node::LogText),
    Failed,
}

/// The host picker can resume a bundled node without choosing a remote alias.
pub(crate) fn enable_option(shell: &Shell) -> bool {
    shell
        .areas
        .local_node
        .snapshot
        .as_ref()
        .is_some_and(|snapshot| {
            matches!(
                snapshot.status,
                Status::Paused | Status::Stopped | Status::Failed | Status::Disconnected
            )
        })
}

pub(crate) fn start(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    crate::local_node::ensure(&shell.prefs, cx);
    let Some(controller) = cx
        .global::<crate::local_node::Managed>()
        .controller
        .as_ref()
    else {
        return;
    };
    let mut state = controller.state();
    let initial = state.borrow_and_update().clone();
    apply(shell, initial, cx);
    cx.spawn_in(window, async move |this, cx| {
        while state.changed().await.is_ok() {
            let snapshot = state.borrow_and_update().clone();
            if this
                .update(cx, |shell, cx| apply(shell, snapshot, cx))
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
}

fn apply(shell: &mut Shell, snapshot: Snapshot, cx: &mut Context<'_, Shell>) {
    shell.areas.local_node.target = None;
    let connection = snapshot.connection();
    shell.areas.local_node.snapshot = Some(snapshot);
    match shell.areas.local_node.tickets.next_ticket() {
        Ok(ticket) => {
            shell.areas.local_node.pending = Some(ticket);
            shell.command(Command::LocalNode(Ask { ticket, connection }));
        }
        Err(_exhausted) => {
            shell.notice = Some(shell.words.local_node().failed().to_owned());
        }
    }
    cx.notify();
}

impl Shell {
    fn read_local_logs(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(controller) = cx
            .try_global::<crate::local_node::Managed>()
            .and_then(|managed| managed.controller.as_ref())
        else {
            return;
        };
        let read = controller.log_tail();
        let Ok(ticket) = self.areas.local_node.tickets.next_ticket() else {
            return;
        };
        self.areas.local_node.logs = Logs::Loading(ticket);
        self.areas.local_node.logs_dialog.focus(window, cx);
        cx.spawn(async move |this, cx| {
            let result = read.await;
            let _closed = this.update(cx, |shell, cx| {
                if !matches!(shell.areas.local_node.logs, Logs::Loading(current) if current == ticket) { return; }
                shell.areas.local_node.logs = match result {
                    Ok(text) => Logs::Ready(text),
                    Err(error) => {tracing::warn!(%error,"explicit local log read failed"); Logs::Failed}
                };
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    /// Exact hydrated local identity for project-open and onboarding consumers.
    pub(crate) fn managed_local_target(&self) -> Option<&LocalTarget> {
        self.areas.local_node.target.as_ref().filter(|target| {
            self.fleet.machines.iter().any(|machine| {
                machine.name == target.machine.as_str()
                    && machine.node == Some(target.node)
                    && machine.reach == vessel_app_view::Reach::Online
            })
        })
    }

    pub(crate) fn local_node_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        if self.areas.local_node.pending != Some(answer.ticket) {
            return;
        }
        // Keep the ticket so a later saved-name collision invalidates this target.
        self.areas.local_node.target = match answer.result {
            Ok(target) => target,
            Err(_failure) => {
                self.notice = Some(self.words.local_node().registration_failed().to_owned());
                None
            }
        };
        cx.notify();
    }

    fn local_node_action(&mut self, action: Action, cx: &mut Context<'_, Self>) {
        let result = cx
            .try_global::<crate::local_node::Managed>()
            .and_then(|managed| managed.controller.as_ref())
            .ok_or(vessel_app_link::local_node::ManagerError::Unavailable)
            .and_then(|controller| controller.request(action));
        if let Err(error) = result {
            tracing::warn!(%error,"local node action was not admitted");
            self.notice = Some(self.words.local_node().failed().to_owned());
        }
        cx.notify();
    }

    pub(crate) fn manage_local_node(&mut self, enabled: bool, cx: &mut Context<'_, Self>) {
        self.prefs.local_node.management = if enabled {
            Management::Enabled
        } else {
            Management::Paused
        };
        self.command(Command::SavePrefs(self.prefs.clone()));
        if enabled {
            self.local_node_action(Action::Start, cx);
        } else {
            // Protect refuses live work; changing automatic intent never grants
            // interruption authority and a reused process always remains alone.
            self.local_node_action(Action::Stop, cx);
        }
        // A refused stop leaves the node running with a visible typed failure.
        cx.notify();
    }

    fn keep_local_node(&mut self, keep: bool, cx: &mut Context<'_, Self>) {
        self.prefs.local_node.after_quit = if keep {
            AfterQuit::KeepRunning
        } else {
            AfterQuit::Stop
        };
        self.command(Command::SavePrefs(self.prefs.clone()));
        cx.notify();
    }
}
