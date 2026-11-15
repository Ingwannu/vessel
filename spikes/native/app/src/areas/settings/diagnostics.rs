//! The existing Settings diagnostic sheet, with independent per-host progress.
//! Reports use safe projections; unavailable producers never become fake successes.
mod draw;
use crate::shell::Shell;
pub(crate) use draw::{dialogs, render};
use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::ui::{ClipboardItem, Context, Window};
use vessel_app_link::{
    Command,
    diagnostics::{Ask, MAX_PARALLEL_HOSTS, Message},
};
use vessel_app_view::Reach;
use vessel_app_view::devices::{MachineKey, Tickets};
use vessel_app_view::diagnostics::collection::{
    Collection, HostCount, MAX_REPORT_HOSTS, Phase, Problem, Progress, ReportText,
};
use vessel_app_view::diagnostics::{self, AppFacts, Renderer};
use vessel_app_view::prefs::Kept;

pub(crate) struct State {
    pub(super) collection: Collection,
    pub(super) dialog: DialogState,
    copied_notice: Option<&'static str>,
    tickets: Tickets,
    pub(super) target: Option<MachineKey>,
}
impl State {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            collection: Collection::default(),
            dialog: DialogState::new(cx),
            copied_notice: None,
            tickets: Tickets::default(),
            target: None,
        }
    }
}
impl Shell {
    /// Only a settled bounded report can cross the clipboard boundary.
    fn copy_diagnostic(&mut self, report: &ReportText, cx: &mut Context<'_, Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(report.as_str().to_owned()));
        let copied = self.words.diagnostics().copied();
        self.areas.settings.diagnostics.copied_notice = Some(copied);
        self.notice = Some(copied.to_owned());
        cx.notify();
    }
    /// Retain L's ownership check so another area's later notice is never replaced.
    pub(crate) fn translate_diagnostic_notice(&mut self) {
        let state = &mut self.areas.settings.diagnostics;
        if let Some(previous) = state.copied_notice {
            let copied = self.words.diagnostics().copied();
            if self.notice.as_deref() == Some(previous) {
                self.notice = Some(copied.to_owned());
            }
            state.copied_notice = Some(copied);
        }
    }

    fn run_diagnostic(
        &mut self,
        target: Option<MachineKey>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.settings.diagnostics.collection.close();
        let facts = app_facts(self, window);
        let client =
            diagnostics::client_report(&facts, HostCount::from_book(self.fleet.machines.len()));
        let ticket = self.areas.settings.diagnostics.tickets.next_ticket();
        let (client, ticket) = match (client, ticket) {
            (Ok(client), Ok(ticket)) => (client, ticket),
            (Err(problem), _) => {
                self.failed_diagnostic(problem, window, cx);
                return;
            }
            (Ok(_), Err(_)) => {
                self.failed_diagnostic(Problem::Exhausted, window, cx);
                return;
            }
        };
        let hosts = host_requests(self, target.as_ref());
        let state = &mut self.areas.settings.diagnostics;
        state.target = target;
        state.collection.start(ticket, client, hosts);
        self.pump_diagnostics();
        self.areas.settings.diagnostics.dialog.focus(window, cx);
        cx.notify();
    }
    fn failed_diagnostic(
        &mut self,
        problem: Problem,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.settings.diagnostics;
        state.collection.refuse(problem);
        state.dialog.focus(window, cx);
        cx.notify();
    }
    pub(crate) fn diagnostic_answer(&mut self, message: Message) {
        match message {
            Message::Answered(answer) => self.areas.settings.diagnostics.collection.answer(
                answer.ticket,
                &answer.machine,
                answer.result,
            ),
            Message::Connected { machine, node } => self
                .areas
                .settings
                .diagnostics
                .collection
                .connected(&machine, node),
        }
        self.pump_diagnostics();
    }
    fn pump_diagnostics(&mut self) {
        let run = &self.areas.settings.diagnostics.collection;
        let Some(ticket) = run.ticket() else {
            return;
        };
        if run.phase() != &Phase::Collecting {
            return;
        }
        let running = run
            .rows()
            .iter()
            .filter(|row| row.progress == Progress::Running)
            .count();
        // Saturation cannot create extra work when a superseded run still holds a link permit.
        let capacity = MAX_PARALLEL_HOSTS.saturating_sub(running);
        let asks: Vec<_> = run
            .rows()
            .iter()
            .filter(|row| row.progress == Progress::Pending)
            .take(capacity)
            .map(|row| Ask {
                machine: row.machine.clone(),
                ticket,
            })
            .collect();
        for ask in asks {
            self.areas
                .settings
                .diagnostics
                .collection
                .running(ticket, &ask.machine);
            self.command(Command::Diagnostics(ask));
        }
    }
    /// Retire pending data after forget, rename or disconnect before accepting more results.
    pub(crate) fn settle_diagnostics(&mut self) {
        if self.areas.settings.diagnostics.collection.phase() == &Phase::Hidden {
            return;
        }
        let invalid: Vec<_> = self
            .areas
            .settings
            .diagnostics
            .collection
            .rows()
            .iter()
            .filter(|row| {
                !self.fleet.machines.iter().any(|machine| {
                    machine.name == row.machine.as_str() && machine.reach == Reach::Online
                })
            })
            .map(|row| row.machine.clone())
            .collect();
        for key in invalid {
            self.areas.settings.diagnostics.collection.invalidate(&key);
        }
    }
}

/// What the app knows of itself now.
fn app_facts(shell: &Shell, window: &Window) -> AppFacts {
    let renderer = window.gpu_specs().map_or(Renderer::Unknown, |specs| {
        if specs.is_software_emulated {
            Renderer::Software {
                device: specs.device_name,
            }
        } else {
            Renderer::Gpu {
                device: specs.device_name,
                driver: specs.driver_name,
            }
        }
    });
    AppFacts {
        collected_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        label: crate::LABEL.to_owned(),
        platform: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        renderer,
        preferences: match &shell.prefs.kept {
            Kept::File { .. } => "file".to_owned(),
            Kept::ThisLaunch { .. } => "this launch only".to_owned(),
        },
        log: None,
        home: None,
    }
}

/// Bound the snapshot before allocating all saved labels into a pending run.
fn host_requests(
    shell: &Shell,
    target: Option<&MachineKey>,
) -> Vec<(MachineKey, Result<(), Problem>)> {
    let mut hosts: Vec<_> = shell
        .fleet
        .machines
        .iter()
        .filter(|machine| target.is_none_or(|key| key.as_str() == machine.name))
        .take(MAX_REPORT_HOSTS.saturating_add(1))
        .map(|machine| {
            (
                MachineKey::from_book(&machine.name),
                match machine.reach {
                    Reach::Online => Ok(()),
                    Reach::Unreachable(_) => Err(Problem::Offline),
                },
            )
        })
        .collect();
    if hosts.is_empty()
        && let Some(key) = target
    {
        hosts.push((key.clone(), Err(Problem::Offline)));
    }
    hosts
}

#[cfg(test)]
mod language_tests;
