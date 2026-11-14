//! Per-machine requests live in view; GPUI retains only picker geometry.
use crate::shell::Shell;
use std::rc::Rc;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_kit::{ComboboxState, select::Anchor};
use vessel_app_link::{
    Command,
    agents::{AgentsError, Answer, Ask, MachineKey},
};
use vessel_app_view::agent_status::{
    pages::{Pages, StartProblem},
    readiness,
};
use vessel_proto::{HarnessRef, OptionId, RequestId};

pub(crate) struct AgentSettings {
    pub(super) machine: Option<MachineKey>,
    pub(super) pages: Pages<AgentsError>,
    pub(super) refused: Option<StartProblem>,
    pub(super) tips: vessel_app_kit::tooltip::Tooltips,
    pub(super) refresh_place: vessel_app_kit::tooltip::Place,
    /// The program's path in the details, when it is cut (its whole text
    /// in a tooltip).
    pub(super) program_place: vessel_app_kit::tooltip::Place,
    /// Each failure alert's tooltip (its original text), by that text.
    pub(super) alert_places:
        std::cell::RefCell<std::collections::HashMap<String, vessel_app_kit::tooltip::Place>>,
    pub(super) defaults: ComboboxState,
    pub(super) default_id: Option<OptionId>,
    pub(super) model_anchor: Anchor,
    pub(super) effort_anchor: Anchor,
    pub(super) mode_anchor: Anchor,
}
impl AgentSettings {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            machine: None,
            pages: Pages::default(),
            refused: None,
            tips: vessel_app_kit::tooltip::Tooltips::new(),
            refresh_place: Rc::default(),
            program_place: Rc::default(),
            alert_places: std::cell::RefCell::default(),
            defaults: ComboboxState::new(window, cx),
            default_id: None,
            model_anchor: Anchor::default(),
            effort_anchor: Anchor::default(),
            mode_anchor: Anchor::default(),
        }
    }
}
impl Shell {
    // A queued mouse event can belong to the previous painted machine.
    // Its control must never redirect an install or a check to the new one.
    pub(super) fn agent_context(&self, shown: Option<&MachineKey>) -> bool {
        self.route == crate::routes::Route::Settings
            && self.areas.settings.section == vessel_app_view::settings::Section::Providers
            && shown.is_some()
            && self.areas.settings.agents.machine.as_ref() == shown
    }

    pub(super) fn agent_back(&mut self, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.settings.agents;
        if let Some(machine) = &state.machine {
            state.pages.select(machine.as_str(), None);
        }
        state.defaults.close();
        state.tips = vessel_app_kit::tooltip::Tooltips::new();
        cx.notify();
    }

    pub(crate) fn agent_load(&mut self, again: bool, cx: &mut Context<'_, Self>) {
        if self.areas.settings.section != vessel_app_view::settings::Section::Providers {
            self.areas.settings.agents.defaults.close();
            self.areas.settings.agents.tips = vessel_app_kit::tooltip::Tooltips::new();
            return;
        }
        let Some(machine) = super::super::choices::chosen_machine(self)
            .map(|machine| MachineKey::from_book(machine.name.as_str()))
        else {
            return;
        };
        let state = &mut self.areas.settings.agents;
        if state.machine.as_ref() != Some(&machine) {
            state.defaults.close();
            state.tips = vessel_app_kit::tooltip::Tooltips::new();
            state.default_id = None;
            state.refused = None;
            state.machine = Some(machine.clone());
        }
        let page = state.pages.get(machine.as_str());
        if page.is_none_or(|page| {
            page.pending().is_none()
                && (again || (page.inventory().is_none() && page.issue().is_none()))
        }) {
            self.agent_ask(Ask::List, cx);
        }
    }
    pub(super) fn agent_ask(&mut self, ask: Ask, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.settings.agents;
        let Some(machine) = state.machine.clone() else {
            return;
        };
        match state.pages.begin(machine.clone(), ask.operation()) {
            Ok(request) => {
                state.refused = None;
                self.command(Command::Agents {
                    machine,
                    request,
                    ask,
                });
            }
            Err(problem) => state.refused = Some(problem),
        }
        cx.notify();
    }
    pub(super) fn agent_select(&mut self, agent: HarnessRef, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.settings.agents;
        let Some(machine) = state.machine.clone() else {
            return;
        };
        let page = state.pages.get(machine.as_str());
        let probe = page
            .and_then(|page| page.inventory())
            .is_some_and(|inventory| {
                inventory.readiness.iter().any(|ready| {
                    ready.agent == agent && readiness::offers_defaults(&ready.installed)
                })
            });
        state.pages.select(machine.as_str(), Some(agent.clone()));
        state.defaults.close();
        state.tips = vessel_app_kit::tooltip::Tooltips::new();
        if probe {
            self.probe_agent((machine.as_str().to_owned(), agent), false);
        }
        cx.notify();
    }
    pub(crate) fn agents_answer(
        &mut self,
        machine: &MachineKey,
        request: RequestId,
        answer: Result<Answer, AgentsError>,
        cx: &mut Context<'_, Self>,
    ) {
        let pages = &mut self.areas.settings.agents.pages;
        match answer {
            Ok(Answer::List(inventory)) => pages.loaded(machine, request, inventory),
            Ok(Answer::Checked(ready)) => pages.checked(machine, request, ready),
            Ok(Answer::Installing(job)) => pages.installing(machine, request, job),
            Ok(Answer::Output(output)) => pages.output(machine, request, &output),
            Ok(Answer::Finished(completion)) => {
                if pages.finished(machine, request, completion) {
                    self.command(Command::Reload);
                    if self.areas.settings.agents.machine.as_ref() == Some(machine) {
                        self.agent_load(true, cx);
                    }
                }
            }
            Err(error) => pages.failed(machine, request, error),
        }
        cx.notify();
    }
}
