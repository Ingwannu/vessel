//! Reuses settings screens; setup owns only the gate, visits and completion.

mod orchestration;
mod render;
mod scope;

use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::Command;
use vessel_app_view::devices::MachineKey;
use vessel_app_view::onboarding::{
    Completion, CompletionRecord, Effect, Layout, Model, SetupHost, State, Step, Visit,
};
use vessel_app_view::prefs::Kept;
use vessel_app_view::settings::Section;

use crate::{routes::Route, shell::Shell};

pub(crate) use render::{dialog, setup_row};

/// One model lives with the window; render never recreates an asynchronous visit.
pub(crate) struct Onboarding {
    model: Model,
    dialog: DialogState,
    shown: Option<Visit>,
    from: Option<Origin>,
    scope: scope::ScopeState,
    integrations: orchestration::Setup,
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
            scope: scope::ScopeState::default(),
            integrations: orchestration::Setup::default(),
        }
    }
}

/// Called after fleet/project events settle, rather than deciding from an empty loading list.
pub(crate) fn prepare(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    if cfg!(any(target_os = "android", target_os = "ios")) {
        return;
    }
    if let State::Open(open) = shell.areas.onboarding.model.state() {
        // A keyboard-accessible background picker cannot redirect the reused
        // settings widgets while this walkthrough owns the selected host.
        shell.areas.settings.machine = Some(open.host().machine().as_str().to_owned());
    }
    scope::prepare(shell);
    let completion = if shell.prefs.onboarding.is_some()
        || matches!(shell.prefs.kept, Kept::ThisLaunch { .. })
    {
        Completion::Completed
    } else {
        Completion::Pending
    };
    let host = SetupHost::Waiting;
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

// C1 must provide the checked guard-on-query Existing/Empty producer before
// automatic setup decides. A legacy Ready cache cannot substitute for that API.

/// A managed launch gets priority before any remote first-host fallback. Wait
/// through startup/registration instead of pinning setup to an unrelated host.
fn selected_host(shell: &Shell) -> Option<&vessel_app_view::Machine> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(target) = shell.managed_local_target() {
            return shell
                .fleet
                .machines
                .iter()
                .find(|machine| machine.name == target.machine.as_str());
        }
        if matches!(
            shell
                .areas
                .local_node
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.status),
            Some(
                vessel_app_view::local_node::Status::Working
                    | vessel_app_view::local_node::Status::Owned(_)
                    | vessel_app_view::local_node::Status::Attached(_)
                    | vessel_app_view::local_node::Status::Kept(_)
            )
        ) {
            // Keep followed by a late quit veto can leave this app open. Only
            // explicit reconnect may restore local admission; never retarget
            // setup to the first unrelated remote machine during that gap.
            return None;
        }
    }
    shell.fleet.machines.first()
}

fn effects_allowed(cx: &gpui::App) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        !crate::quit::is_finalizing(cx)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _browser = cx;
        true
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
    let (visit, step, target) = (open.visit(), open.step(), open.host().clone());
    let host = target.machine().as_str().to_owned();
    orchestration::leave(shell);
    if shell.areas.onboarding.from.is_none() {
        orchestration::opened(shell, cx);
    }
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
    scope::bind(shell, target, visit);
}

impl Shell {
    /// Explicit reopen keeps completed storage intact until setup closes again.
    pub(crate) fn reopen_setup(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        scope::manual(self, window);
        cx.notify();
    }

    fn setup_move(
        &mut self,
        visit: Visit,
        step: Option<Step>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !matches!(self.areas.onboarding.model.state(), State::Open(open) if open.visit() == visit)
        {
            return;
        }
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
        if !scope::actionable(self, cx) {
            return;
        }
        if matches!(self.areas.onboarding.model.state(), State::Open(open) if open.visit() == visit && open.step() == Step::Orchestration)
        {
            orchestration::apply(self, visit, window, cx);
            return;
        }
        self.advance_setup(visit, window, cx);
    }

    fn advance_setup(&mut self, visit: Visit, window: &mut Window, cx: &mut Context<'_, Self>) {
        if !scope::actionable(self, cx) {
            return;
        }
        match self.areas.onboarding.model.next(visit) {
            Ok(Effect::Complete) => {
                scope::release(self, visit);
                self.persist_setup_completion(window, cx);
            }
            Ok(Effect::None) => enter_step(self, window, cx),
            Err(_exhausted) => self.notice = Some(self.words.desktop().setup_failed().to_owned()),
        }
        cx.notify();
    }

    fn setup_complete(&mut self, visit: Visit, window: &mut Window, cx: &mut Context<'_, Self>) {
        if !matches!(self.areas.onboarding.model.state(), State::Open(open) if open.visit() == visit)
        {
            return;
        }
        scope::release(self, visit);
        let Effect::Complete = self.areas.onboarding.model.complete() else {
            return;
        };
        self.persist_setup_completion(window, cx);
    }

    fn persist_setup_completion(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.onboarding.dialog.dismissed(window, cx);
        orchestration::leave(self);
        orchestration::closed(self);
        self.prefs.onboarding = Some(CompletionRecord {
            completed_at: chrono::Utc::now(),
        });
        self.command(Command::SavePrefs(Box::new(self.prefs.clone())));
        self.areas.onboarding.shown = None;
        if let Some(origin) = self.areas.onboarding.from.take() {
            self.areas.settings.machine = origin.machine.map(|machine| machine.as_str().to_owned());
            self.settings_section(origin.section, cx);
            self.go(origin.route, window, cx);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests;

pub(crate) fn orchestration_content(
    shell: &Shell,
    tokens: &vessel_app_kit::Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> gpui::AnyElement {
    orchestration::render(shell, tokens, compact, cx)
}
