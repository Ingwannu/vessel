//! Project script controls reuse the workspace channel and approval card.
//! The node owns all processes; leaving this screen never stops a service.
mod draw;
mod ports;
use crate::routes::Route;
use crate::shell::Shell;
pub(super) use draw::render;
use vessel_app_kit::{
    TextField,
    ui::{Context, Window},
};
use vessel_app_link::workspace::{Ask, ScriptAnswer, ScriptAsk};
use vessel_app_view::machine_book::Name;
use vessel_app_view::settings::Section;
use vessel_app_view::workspace::scripts::{Action, Issue, Model, Preview, REFRESH_EVERY, Target};
use vessel_proto::ProjectId;

/// Controls survive a detail refresh, while model keys survive every reopen.
pub(super) struct Scripts {
    pub(super) model: Model,
    pub(super) host: TextField,
    polling: bool,
    // Navigation intent only. This pair grants no RPC admission.
    selection: Option<(Name, ProjectId)>,
}
impl Scripts {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            model: Model::default(),
            host: TextField::new(
                "",
                |_shell: &mut Shell, _text, _window, cx| cx.notify(),
                window,
                cx,
            ),
            polling: false,
            selection: None,
        }
    }
}
impl Shell {
    pub(super) fn open_scripts(
        &mut self,
        machine: &str,
        project: ProjectId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Ok(machine) = Name::parse(machine) else {
            return;
        };
        self.areas.workspace.scripts.host.clear(window, cx);
        self.areas.workspace.scripts.host.set_placeholder(
            self.words.scripts().host_placeholder(),
            window,
            cx,
        );
        self.areas.workspace.scripts.selection = Some((machine, project));
        self.areas.workspace.scripts.model.select(None);
        self.read_scripts();
        if !self.areas.workspace.scripts.polling {
            self.areas.workspace.scripts.polling = true;
            Self::poll_scripts(cx);
        }
    }
    fn scripts_visible(&self) -> bool {
        self.route == Route::Settings
            && self.areas.settings.pushed
            && self.areas.settings.section == Section::Projects
            && self
                .areas
                .workspace
                .scripts
                .selection
                .as_ref()
                .is_some_and(|(machine, project)| {
                    self.projects_machine().as_deref() == Some(machine.as_str())
                        && self
                            .areas
                            .workspace
                            .page
                            .detail
                            .as_ref()
                            .is_some_and(|detail| detail.project == *project)
                })
    }
    fn selected_scripts_target(&self) -> Option<Target> {
        let (machine, project) = self.areas.workspace.scripts.selection.as_ref()?;
        let context = self
            .fleet
            .machines
            .iter()
            .find(|snapshot| snapshot.name == machine.as_str())?
            .connection_context()?;
        Some(Target {
            context,
            project: *project,
        })
    }
    fn scripts_target_is_current(&self, expected: &Target) -> bool {
        self.scripts_visible()
            && self.selected_scripts_target().as_ref() == Some(expected)
            && self.areas.workspace.scripts.model.target.as_ref() == Some(expected)
    }
    fn read_scripts(&mut self) {
        let current = self.selected_scripts_target();
        let model = &mut self.areas.workspace.scripts.model;
        if model.target != current {
            model.select(current);
        }
        let Some(target) = model.target.clone() else {
            return;
        };
        let Some(key) = model.begin_read() else {
            return;
        };
        self.ask(
            target.machine(),
            Ask::Scripts(ScriptAsk::Read {
                target: target.clone(),
                key,
            }),
        );
    }
    fn poll_scripts(cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(REFRESH_EVERY).await;
                if this
                    .update(cx, |shell, _cx| {
                        if shell.scripts_visible() {
                            shell.read_scripts();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }
    pub(super) fn scripts_answer(&mut self, answer: ScriptAnswer, cx: &mut Context<'_, Self>) {
        let returned = match &answer {
            ScriptAnswer::Read { target, .. } | ScriptAnswer::Written { target, .. } => target,
        };
        if !self.scripts_target_is_current(returned) {
            return;
        }
        let (accepted, written) = match answer {
            ScriptAnswer::Read {
                target,
                key,
                result,
            } => {
                let commands = result
                    .as_ref()
                    .ok()
                    .map(|snapshot| snapshot.scripts.commands.clone());
                let accepted = self
                    .areas
                    .workspace
                    .scripts
                    .model
                    .accept_read(&target, key, result);
                if accepted
                    && self.scripts_visible()
                    && let Some(commands) = commands
                {
                    self.project_commands(target.machine(), target.project, None, Ok(commands));
                }
                (accepted, false)
            }
            ScriptAnswer::Written {
                target,
                key,
                result,
            } => {
                let accepted = self
                    .areas
                    .workspace
                    .scripts
                    .model
                    .accept_write(&target, key, result);
                (accepted, true)
            }
        };
        if accepted {
            if written {
                if self.scripts_visible() {
                    self.read_scripts();
                }
            }
            cx.notify();
        }
    }
    pub(super) fn script_action(
        &mut self,
        expected: &Target,
        action: Action,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.scripts_target_is_current(expected) {
            tracing::debug!("script action ignored after its view changed");
            return;
        }
        let model = &mut self.areas.workspace.scripts.model;
        let Some(target) = model.target.clone() else {
            return;
        };
        let Some(key) = model.begin_write(action.clone()) else {
            tracing::debug!("script action requires fresh current state");
            return;
        };
        tracing::debug!("script lifecycle request dispatched");
        self.ask(
            target.machine(),
            Ask::Scripts(ScriptAsk::Write {
                target: target.clone(),
                key,
                action,
            }),
        );
        cx.notify();
    }
    pub(super) fn script_preview(
        &mut self,
        expected: &Target,
        preview: &Preview,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.scripts_target_is_current(expected) {
            return;
        }
        let scripts = &mut self.areas.workspace.scripts;
        let result = scripts.model.preview_url(preview, &scripts.host.text(cx));
        match result {
            Ok(url) => {
                tracing::debug!("opening the observed script listener in the system browser");
                cx.open_url(url.as_str());
            }
            Err(problem) => {
                tracing::debug!(?problem, "script preview refused after socket recheck");
                scripts.model.issue = Some(Issue::Preview(problem));
            }
        }
        cx.notify();
    }
    pub(super) fn refresh_scripts(
        &mut self,
        expected: &Option<Target>,
        cx: &mut Context<'_, Self>,
    ) {
        if self.scripts_visible() && &self.areas.workspace.scripts.model.target == expected {
            self.read_scripts();
            cx.notify();
        }
    }
}
