//! Project script controls reuse the workspace channel and approval card.
//! The node owns all processes; leaving this screen never stops a service.
mod draw;
mod panel;
mod ports;
use crate::routes::Route;
use crate::shell::Shell;
pub(super) use draw::render;
pub(crate) use panel::render as render_panel;
pub(crate) use panel::retirement::start;
use vessel_app_kit::{
    TextField,
    ui::{Context, Window},
};
use vessel_app_link::workspace::{Ask, ScriptAnswer, ScriptAsk};
use vessel_app_view::machine_book::Name;
use vessel_app_view::settings::Section;
use vessel_app_view::workspace::scripts::{
    Action, ActionTicket, Issue, Model, Preview, REFRESH_EVERY, Surface, Target,
};
use vessel_proto::ProjectId;

/// Both destinations pass the same final owner/publication/socket check.
#[derive(Clone, Copy)]
pub(super) enum UrlAction {
    Open,
    Copy,
}
impl UrlAction {
    fn deliver(self, url: &str, cx: &mut Context<'_, Shell>) {
        match self {
            Self::Open => cx.open_url(url),
            Self::Copy => cx.write_to_clipboard(vessel_app_kit::ui::ClipboardItem::new_string(
                url.to_owned(),
            )),
        }
    }
}

/// Controls survive a detail refresh, while model keys survive every reopen.
pub(super) struct Scripts {
    pub(super) model: Model,
    pub(super) host: TextField,
    polling: bool,
    retirement_ready: bool,
    panel_keys: std::cell::RefCell<vessel_app_view::workspace::git::RequestKeys>,
    panels: std::cell::RefCell<
        std::collections::BTreeMap<
            (
                vessel_app_view::layout::LayoutKey,
                vessel_app_view::layout::TabId,
            ),
            panel::Controls,
        >,
    >,
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
            retirement_ready: false,
            panel_keys: std::cell::RefCell::default(),
            panels: std::cell::RefCell::default(),
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
        self.read_scripts(window, cx);
        if !self.areas.workspace.scripts.polling {
            self.areas.workspace.scripts.polling = true;
            Self::poll_scripts(window, cx);
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
            place: vessel_proto::scripts::ScriptPlace::Project { project: *project },
            surface: Surface::Project,
        })
    }
    fn scripts_target_is_current(&self, expected: &Target) -> bool {
        self.scripts_visible()
            && self.selected_scripts_target().as_ref() == Some(expected)
            && self.areas.workspace.scripts.model.target.as_ref() == Some(expected)
    }
    fn read_scripts(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let current = self.selected_scripts_target();
        let scripts = &mut self.areas.workspace.scripts;
        if scripts.model.target != current {
            scripts.host.clear(window, cx);
            scripts.model.select(current);
        }
        let model = &mut scripts.model;
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
    fn poll_scripts(window: &mut Window, cx: &mut Context<'_, Self>) {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(REFRESH_EVERY).await;
                if this
                    .update_in(cx, |shell, window, cx| {
                        if window.is_window_active() && shell.scripts_visible() {
                            shell.read_scripts(window, cx);
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
        if matches!(returned.surface, Surface::Panel { .. }) {
            self.script_panel_answer(answer, cx);
            return;
        }
        if !self.scripts_target_is_current(returned) {
            return;
        }
        let accepted = match answer {
            ScriptAnswer::Read {
                target,
                key,
                result,
            } => self
                .areas
                .workspace
                .scripts
                .model
                .accept_read(&target, key, result),
            ScriptAnswer::Written {
                target,
                key,
                result,
            } => self
                .areas
                .workspace
                .scripts
                .model
                .accept_write(&target, key, result),
        };
        if accepted {
            // The window-aware timer performs reconciliation. A late write
            // receipt never starts polling from an inactive OS window.
            cx.notify();
        }
    }
    pub(super) fn script_action(
        &mut self,
        ticket: &ActionTicket,
        action: Action,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.scripts_target_is_current(&ticket.target) {
            tracing::debug!("script action ignored after its view changed");
            return;
        }
        let model = &mut self.areas.workspace.scripts.model;
        let Some(target) = model.target.clone() else {
            return;
        };
        let Some(key) = model.begin_write_at(ticket, action.clone()) else {
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
        action: UrlAction,
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
                action.deliver(url.as_str(), cx);
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
        expected: Option<&Target>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.scripts_visible() && self.areas.workspace.scripts.model.target.as_ref() == expected
        {
            self.read_scripts(window, cx);
            cx.notify();
        }
    }
}

impl Shell {
    fn script_card_is_current(&self, target: &Target) -> bool {
        match target.surface {
            Surface::Project => self.scripts_target_is_current(target),
            Surface::Panel { .. } => self.script_panel_is_current(target),
        }
    }
    fn script_action_for(
        &mut self,
        ticket: &ActionTicket,
        action: Action,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !window.is_window_active() {
            return;
        }
        match ticket.target.surface {
            Surface::Project => self.script_action(ticket, action, cx),
            Surface::Panel { .. } => self.script_panel_action(ticket, action, window, cx),
        }
    }
    fn script_preview_for(
        &mut self,
        target: &Target,
        preview: &Preview,
        action: UrlAction,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !window.is_window_active() {
            return;
        }
        match target.surface {
            Surface::Project => self.script_preview(target, preview, action, cx),
            Surface::Panel { .. } => self.script_panel_preview(target, preview, action, window, cx),
        }
    }
    fn refresh_scripts_for(
        &mut self,
        surface: &Surface,
        target: Option<&Target>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !window.is_window_active() {
            return;
        }
        match surface {
            Surface::Project => self.refresh_scripts(target, window, cx),
            Surface::Panel { .. } => self.refresh_script_panel(surface, target, window, cx),
        }
    }
}
