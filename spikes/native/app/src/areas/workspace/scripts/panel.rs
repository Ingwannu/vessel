//! Passive layout integration. Tabs own display state; the node owns resources.
//! Exact legacy session keys use existing RPCs. Canonical workspace execution
//! stays unavailable until its reviewed registry/runtime contract is supplied.
pub(super) mod retirement;
use super::draw::{Card, content};
use crate::areas::layout::registry::PanelContext;
use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div,
};
use vessel_app_kit::{TextField, Tokens};
use vessel_app_link::workspace::{Ask, ScriptAnswer, ScriptAsk};
use vessel_app_view::layout::{LayoutKey, TabId};
use vessel_app_view::workspace::scripts::{
    Action, ActionTicket, Issue, Model, Preview, REFRESH_EVERY, Surface, Target,
};

pub(super) struct Controls {
    model: Model,
    host: TextField,
    surface: Surface,
    retired: bool,
    last_write: Option<vessel_app_view::workspace::scripts::RetiredWrite>,
}
impl Controls {
    fn new(surface: Surface, window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            model: Model::default(),
            surface,
            retired: false,
            last_write: None,
            host: TextField::new(
                "",
                |_shell: &mut Shell, _text, _window, cx| cx.notify(),
                window,
                cx,
            ),
        }
    }
}
fn key(surface: &Surface) -> Option<(&LayoutKey, TabId)> {
    match surface {
        Surface::Panel { workspace, tab, .. } => Some((workspace, *tab)),
        Surface::Project => None,
    }
}
fn target(shell: &Shell, surface: &Surface) -> Option<Target> {
    let (workspace, _tab) = key(surface)?;
    let LayoutKey::Session { machine, .. } = workspace else {
        return None;
    };
    let context = shell
        .fleet
        .machines
        .iter()
        .find(|entry| entry.name == machine.as_str())?
        .connection_context()?;
    Target::legacy_panel(context, surface.clone())
}

// Current equality is only an extra refusal condition. Synchronous owner
// retirement invalidates the old instance before an ABA target can match again.
fn target_matches(shell: &Shell, workspace: &LayoutKey, tab: TabId) -> bool {
    shell.areas.workspace.scripts.retirement_ready
        && shell.layout_tab_matches(workspace, tab, &vessel_app_view::layout::Target::Scripts)
}

pub(crate) fn render(
    shell: &Shell,
    panel: &PanelContext<'_>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    if matches!(panel.workspace, LayoutKey::Workspace { .. })
        || !target_matches(shell, panel.workspace, panel.tab)
    {
        return super::ports::notice(shell.words.layout().panel_unavailable(), tokens)
            .into_any_element();
    }
    let address = (panel.workspace.clone(), panel.tab);
    if let Some(notice) = retirement::draw_retired(shell, &address, tokens, cx) {
        return notice;
    }
    let inserted = {
        let mut states = shell.areas.workspace.scripts.panels.borrow_mut();
        if !states.contains_key(&address) && states.len() >= retirement::MAX_PANEL_STATES {
            return super::ports::notice(shell.words.scripts().retained_limit(), tokens)
                .into_any_element();
        }
        if let std::collections::btree_map::Entry::Vacant(entry) = states.entry(address.clone()) {
            let Ok(instance) = shell
                .areas
                .workspace
                .scripts
                .panel_keys
                .borrow_mut()
                .issue()
            else {
                return super::ports::notice(shell.words.layout().panel_unavailable(), tokens)
                    .into_any_element();
            };
            let surface = Surface::Panel {
                workspace: panel.workspace.clone(),
                tab: panel.tab,
                instance,
            };
            let _new = entry.insert(Controls::new(surface, window, cx));
            true
        } else {
            false
        }
    };
    let surface = {
        let states = shell.areas.workspace.scripts.panels.borrow();
        let Some(controls) = states.get(&address) else {
            return div().into_any_element();
        };
        controls.surface.clone()
    };
    if inserted {
        poll(&surface, window, cx);
    }
    let states = shell.areas.workspace.scripts.panels.borrow();
    let Some(controls) = states.get(&address) else {
        return div().into_any_element();
    };
    div()
        .id(vessel_app_kit::ui::ElementId::Name(
            format!("scripts-body-{:?}-{}", panel.workspace, panel.tab.get()).into(),
        ))
        .size_full()
        .min_w_0()
        .min_h_0()
        .overflow_y_scroll()
        .child(content(
            &Card {
                shell,
                model: &controls.model,
                host: &controls.host,
                surface: &surface,
            },
            tokens,
            cx,
        ))
        .into_any_element()
}
fn poll(surface: &Surface, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let observed = surface.clone();
    cx.spawn_in(window, async move |this, cx| {
        loop {
            let keep = this
                .update_in(cx, |shell, window, cx| {
                    let Some((workspace, tab)) = key(&observed) else {
                        return false;
                    };
                    let same = shell
                        .areas
                        .workspace
                        .scripts
                        .panels
                        .borrow()
                        .get(&(workspace.clone(), tab))
                        .is_some_and(|controls| controls.surface == observed);
                    if !same {
                        return false;
                    }
                    if !shell.layout_tab_exists(workspace, tab) {
                        return false;
                    }
                    if window.is_window_active()
                        && shell.layout_panel_visible(workspace, tab, window)
                    {
                        shell.read_script_panel(&observed, window, cx);
                        cx.notify();
                    }
                    true
                })
                .unwrap_or(false);
            if !keep {
                break;
            }
            cx.background_executor().timer(REFRESH_EVERY).await;
        }
    })
    .detach();
}
impl Shell {
    pub(super) fn script_panel_is_current(&self, expected: &Target) -> bool {
        let Some((workspace, tab)) = key(&expected.surface) else {
            return false;
        };
        target_matches(self, workspace, tab)
            && self.layout_tab_exists(workspace, tab)
            && target(self, &expected.surface).as_ref() == Some(expected)
            && self
                .areas
                .workspace
                .scripts
                .panels
                .borrow()
                .get(&(workspace.clone(), tab))
                .is_some_and(|controls| {
                    !controls.retired
                        && controls.model.target.as_ref() == Some(expected)
                        && controls.surface == expected.surface
                })
    }
    fn script_panel_admits(&self, expected: &Target, window: &Window) -> bool {
        key(&expected.surface).is_some_and(|(workspace, tab)| {
            window.is_window_active() && self.layout_panel_visible(workspace, tab, window)
        }) && self.script_panel_is_current(expected)
    }
    pub(super) fn read_script_panel(
        &mut self,
        surface: &Surface,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let current = target(self, surface);
        let Some((workspace, tab)) = key(surface) else {
            return;
        };
        if !target_matches(self, workspace, tab) {
            return;
        }
        let request = {
            let mut states = self.areas.workspace.scripts.panels.borrow_mut();
            let Some(controls) = states.get_mut(&(workspace.clone(), tab)) else {
                return;
            };
            if controls.retired || &controls.surface != surface {
                return;
            }
            if controls.model.target != current {
                if controls.model.writing.is_some() {
                    retirement::withdraw(controls);
                    return;
                }
                controls.host.clear(window, cx);
                controls.model.select(current);
            }
            let Some(target) = controls.model.target.clone() else {
                return;
            };
            controls.model.begin_read().map(|key| (target, key))
        };
        if let Some((target, key)) = request {
            self.ask(
                target.machine(),
                Ask::Scripts(ScriptAsk::Read {
                    target: target.clone(),
                    key,
                }),
            );
        }
    }
    pub(super) fn script_panel_answer(&mut self, answer: ScriptAnswer, cx: &mut Context<'_, Self>) {
        if retirement::accept(self, &answer, cx) {
            return;
        }
        let expected = match &answer {
            ScriptAnswer::Read { target, .. } | ScriptAnswer::Written { target, .. } => target,
        };
        if !self.script_panel_is_current(expected) {
            return;
        }
        let surface = expected.surface.clone();
        let Some((workspace, tab)) = key(&surface) else {
            return;
        };
        let accepted = {
            let mut states = self.areas.workspace.scripts.panels.borrow_mut();
            let Some(controls) = states.get_mut(&(workspace.clone(), tab)) else {
                return;
            };
            match answer {
                ScriptAnswer::Read {
                    target,
                    key,
                    result,
                } => controls.model.accept_read(&target, key, result),
                ScriptAnswer::Written {
                    target,
                    key,
                    result,
                } => controls.model.accept_write(&target, key, result),
            }
        };
        // No polling or automatic replay is caused by a hidden receipt. The
        // visible-panel timer performs the next discovery after reconciliation.
        if accepted {
            cx.notify();
        }
    }
    pub(super) fn script_panel_action(
        &mut self,
        ticket: &ActionTicket,
        action: Action,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        let expected = &ticket.target;
        if !self.script_panel_admits(expected, window) {
            return;
        }
        let Some((workspace, tab)) = key(&expected.surface) else {
            return;
        };
        let request = {
            let mut states = self.areas.workspace.scripts.panels.borrow_mut();
            states
                .get_mut(&(workspace.clone(), tab))
                .and_then(|controls| {
                    let key = controls.model.begin_write_at(ticket, action.clone())?;
                    // A new explicit intention supersedes the acknowledged old
                    // receipt only after the successor obtained a fresh read.
                    controls.last_write = None;
                    Some(key)
                })
        };
        if let Some(key) = request {
            self.ask(
                expected.machine(),
                Ask::Scripts(ScriptAsk::Write {
                    target: expected.clone(),
                    key,
                    action,
                }),
            );
            cx.notify();
        }
    }
    pub(super) fn script_panel_preview(
        &mut self,
        expected: &Target,
        preview: &Preview,
        action: super::UrlAction,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.script_panel_admits(expected, window) {
            return;
        }
        let Some((workspace, tab)) = key(&expected.surface) else {
            return;
        };
        let mut states = self.areas.workspace.scripts.panels.borrow_mut();
        let Some(controls) = states.get_mut(&(workspace.clone(), tab)) else {
            return;
        };
        match controls.model.preview_url(preview, &controls.host.text(cx)) {
            Ok(url) => action.deliver(url.as_str(), cx),
            Err(problem) => controls.model.issue = Some(Issue::Preview(problem)),
        }
        cx.notify();
    }
    pub(super) fn refresh_script_panel(
        &mut self,
        surface: &Surface,
        expected: Option<&Target>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some((workspace, tab)) = key(surface) else {
            return;
        };
        if !window.is_window_active() || !self.layout_panel_visible(workspace, tab, window) {
            return;
        }
        let same = self
            .areas
            .workspace
            .scripts
            .panels
            .borrow()
            .get(&(workspace.clone(), tab))
            .is_some_and(|controls| controls.model.target.as_ref() == expected);
        if same {
            self.read_script_panel(surface, window, cx);
            cx.notify();
        }
    }
}

impl Shell {
    pub(super) fn script_panel_reveal(
        &mut self,
        ticket: &ActionTicket,
        terminal: vessel_proto::TerminalId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.script_panel_admits(&ticket.target, window) {
            return;
        }
        let Some((workspace, tab)) = key(&ticket.target.surface) else {
            return;
        };
        let allowed = self
            .areas
            .workspace
            .scripts
            .panels
            .borrow()
            .get(&(workspace.clone(), tab))
            .is_some_and(|controls| {
                controls.model.action_ticket().as_ref() == Some(ticket)
                    && controls.model.read.ready().is_some_and(|snapshot| {
                        snapshot.scripts.runs.iter().any(|run| run.id == terminal)
                    })
            });
        if allowed {
            let _opened = self.layout_open_target(
                workspace,
                vessel_app_view::layout::Target::Terminal(terminal),
                vessel_app_view::layout::OpenIntent::Reveal,
                vessel_app_view::layout::Placement::Focused,
                window,
                cx,
            );
        }
    }
}
