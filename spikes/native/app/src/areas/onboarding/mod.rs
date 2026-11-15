//! Reuses settings screens; setup owns only the gate, visits and completion.

mod render;

use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_view::Reach;
use vessel_app_view::devices::MachineKey;
use vessel_app_view::onboarding::{
    Completion, CompletionRecord, Effect, Layout, Model, SetupHost, State, Step, Visit,
};
use vessel_app_view::prefs::Kept;
use vessel_app_view::settings::Section;
use vessel_app_view::workspace::projects::ProjectsView;

use crate::{routes::Route, shell::Shell};

pub(crate) use render::{dialog, setup_row};

/// One model lives with the window; render never recreates an asynchronous visit.
pub(crate) struct Onboarding {
    model: Model,
    dialog: DialogState,
    shown: Option<Visit>,
    from: Option<Origin>,
}

/// Setup temporarily borrows existing settings owners without changing the
/// person's prior route, section or host when the walkthrough closes.
struct Origin {
    route: Route,
    section: Section,
    machine: Option<MachineKey>,
}

impl Onboarding {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            model: Model::default(),
            dialog: DialogState::new(cx),
            shown: None,
            from: None,
        }
    }
}

/// Called after fleet/project events settle, rather than deciding from an empty loading list.
pub(crate) fn prepare(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    if cfg!(any(target_os = "android", target_os = "ios")) {
        return;
    }
    let completion = if shell.prefs.onboarding.is_some()
        || matches!(shell.prefs.kept, Kept::ThisLaunch { .. })
    {
        Completion::Completed
    } else {
        Completion::Pending
    };
    let host = setup_host(shell);
    let result = shell
        .areas
        .onboarding
        .model
        .decide(completion, host, layout(window));
    if result.is_err() {
        shell.notice = Some(shell.words.desktop().setup_failed().to_owned());
    }
    enter_step(shell, window, cx);
}

fn setup_host(shell: &Shell) -> SetupHost {
    if !shell.loaded {
        return SetupHost::Waiting;
    }
    // Managed-node registration will take precedence when its controller has
    // authenticated it; otherwise the reference selects the first saved host.
    let Some(machine) = shell.fleet.machines.first() else {
        return SetupHost::Waiting;
    };
    if machine.reach != Reach::Online {
        return SetupHost::Waiting;
    }
    match shell.areas.workspace.projects.get(&machine.name) {
        Some(ProjectsView::Ready(list)) if list.projects.is_empty() => {
            SetupHost::Empty(MachineKey::from_book(&machine.name))
        }
        Some(ProjectsView::Ready(_)) => SetupHost::Existing,
        Some(ProjectsView::Loading | ProjectsView::Failed(_) | ProjectsView::Older) | None => {
            SetupHost::Waiting
        }
    }
}

fn layout(window: &Window) -> Layout {
    if crate::areas::draft::compact(window) {
        Layout::Compact
    } else {
        Layout::Wide
    }
}

fn section(step: Step) -> Section {
    match step {
        Step::Appearance => Section::Appearance,
        Step::Providers => Section::Providers,
        Step::Projects => Section::Projects,
        Step::Orchestration => Section::Agents,
        Step::Phone => Section::Devices,
    }
}

fn enter_step(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let State::Open(open) = shell.areas.onboarding.model.state() else {
        return;
    };
    if shell.areas.onboarding.shown == Some(open.visit()) {
        return;
    }
    let (visit, step, host) = (open.visit(), open.step(), open.host().as_str().to_owned());
    let area = &mut shell.areas.onboarding;
    if area.from.is_none() {
        area.from = Some(Origin {
            route: shell.route,
            section: shell.areas.settings.section,
            machine: shell
                .areas
                .settings
                .machine
                .as_deref()
                .map(MachineKey::from_book),
        });
    }
    area.shown = Some(visit);
    area.dialog.focus(window, cx);
    shell.areas.settings.machine = Some(host);
    shell.route = Route::Settings;
    shell.settings_section(section(step), cx);
}

impl Shell {
    /// Explicit reopen keeps completed storage intact until setup closes again.
    pub(crate) fn reopen_setup(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(machine) = self.fleet.machines.first() else {
            return;
        };
        let host = MachineKey::from_book(&machine.name);
        if self
            .areas
            .onboarding
            .model
            .reopen(host, layout(window))
            .is_err()
        {
            self.notice = Some(self.words.desktop().setup_failed().to_owned());
        }
        enter_step(self, window, cx);
        cx.notify();
    }

    fn setup_move(&mut self, step: Option<Step>, window: &mut Window, cx: &mut Context<'_, Self>) {
        let result = match step {
            Some(step) => self.areas.onboarding.model.go_to(step),
            None => self.areas.onboarding.model.back(),
        };
        if result.is_err() {
            self.notice = Some(self.words.desktop().setup_failed().to_owned());
        }
        enter_step(self, window, cx);
        cx.notify();
    }

    fn setup_continue(&mut self, visit: Visit, window: &mut Window, cx: &mut Context<'_, Self>) {
        match self.areas.onboarding.model.next(visit) {
            Ok(Effect::Complete) => self.setup_complete(window, cx),
            Ok(Effect::None) => enter_step(self, window, cx),
            Err(_exhausted) => self.notice = Some(self.words.desktop().setup_failed().to_owned()),
        }
        cx.notify();
    }

    fn setup_complete(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Effect::Complete = self.areas.onboarding.model.complete() else {
            return;
        };
        self.prefs.onboarding = Some(CompletionRecord {
            completed_at: chrono::Utc::now(),
        });
        self.command(Command::SavePrefs(self.prefs.clone()));
        self.areas.onboarding.shown = None;
        if let Some(origin) = self.areas.onboarding.from.take() {
            self.areas.settings.machine = origin.machine.map(|machine| machine.as_str().to_owned());
            self.settings_section(origin.section, cx);
            self.go(origin.route, window, cx);
        }
        cx.notify();
    }
}
