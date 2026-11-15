//! The existing Settings diagnostic sheet, with independent per-host progress.
//! Reports use safe projections; unavailable producers never become fake successes.
mod draw;
use crate::shell::Shell;
pub(crate) use draw::{dialogs, render};
use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{
    Command,
    diagnostics::{Answer, Ask},
};
use vessel_app_view::Reach;
use vessel_app_view::devices::{MachineKey, Tickets};
use vessel_app_view::diagnostics::collection::{Collection, Phase, Problem, Progress, ReportText};
use vessel_app_view::diagnostics::{self, AppFacts, Renderer};
use vessel_app_view::prefs::Kept;

pub(crate) struct State {
    pub(super) collection: Collection,
    pub(super) dialog: DialogState,
    tickets: Tickets,
    pub(super) target: Option<MachineKey>,
}
impl State {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            collection: Collection::default(),
            dialog: DialogState::new(cx),
            tickets: Tickets::default(),
            target: None,
        }
    }
}
impl Shell {
    fn run_diagnostic(
        &mut self,
        target: Option<MachineKey>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let facts = app_facts(self, window);
        let client = ReportText::new(&diagnostics::report(&facts, &[]));
        let ticket = self.areas.settings.diagnostics.tickets.next_ticket();
        let (client, ticket) = match (client, ticket) {
            (Ok(client), Ok(ticket)) => (client, ticket),
            (Err(problem), _) => {
                self.notice = Some(diagnostics::words::problem(problem, self.words).to_owned());
                return;
            }
            (Ok(_), Err(_)) => {
                self.notice =
                    Some(diagnostics::words::problem(Problem::Exhausted, self.words).to_owned());
                return;
            }
        };
        let hosts: Vec<_> = self
            .fleet
            .machines
            .iter()
            .filter(|machine| {
                target
                    .as_ref()
                    .is_none_or(|key| key.as_str() == machine.name)
            })
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
        let state = &mut self.areas.settings.diagnostics;
        state.target = target;
        state
            .collection
            .start(ticket, chrono::Utc::now(), client, hosts);
        let asks: Vec<_> = state
            .collection
            .rows()
            .iter()
            .filter(|row| row.progress == Progress::Pending)
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
        self.areas.settings.diagnostics.dialog.focus(window, cx);
        cx.notify();
    }
    pub(crate) fn diagnostic_answer(&mut self, answer: Answer) {
        self.areas.settings.diagnostics.collection.answer(
            answer.ticket,
            &answer.machine,
            answer.result,
        );
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
