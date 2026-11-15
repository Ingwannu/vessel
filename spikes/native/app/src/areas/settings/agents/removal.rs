//! Legacy added definitions use the shared `ProviderRow` menu and confirm dialog.
//! The view owns deletion authority; this area retains the painted decision's
//! identity so a delayed click cannot confirm or dismiss a newer dialog.
use crate::shell::Shell;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
use vessel_app_kit::{
    DialogState,
    menu::{MenuState, TriggerAnchor},
    ui::{App, Context, Window},
};
use vessel_app_view::agent_status::{
    pages::MachineKey,
    removal::{Confirmation, Refused},
};
use vessel_proto::{HarnessRef, NodeId};
mod draw;
pub(super) use draw::{dialog, menu, notice, trigger};
struct Decision {
    machine: MachineKey,
    node: NodeId,
    agent: HarnessRef,
    label: String,
    title: RefCell<vessel_app_kit::ui::SharedString>,
    confirmation: RefCell<Option<Confirmation>>,
}
pub(super) struct State {
    menu: MenuState,
    target: Option<Rc<Decision>>,
    anchors: RefCell<HashMap<String, TriggerAnchor>>,
    dialog: DialogState,
    confirmation: Option<Rc<Decision>>,
    problem: Option<Refused>,
    word_error: Option<vessel_app_view::localization::FormatError>,
    restore: Cell<bool>,
}
impl State {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            menu: MenuState::new(cx),
            target: None,
            anchors: RefCell::default(),
            dialog: DialogState::new(cx),
            confirmation: None,
            problem: None,
            word_error: None,
            restore: Cell::new(false),
        }
    }
    pub(super) fn reset(&mut self) {
        self.menu.close();
        self.target = None;
        self.anchors.borrow_mut().clear();
        if self.confirmation.take().is_some() {
            self.restore.set(true);
        }
        self.problem = None;
        self.word_error = None;
    }
}
impl Shell {
    fn agent_open_actions(
        &mut self,
        machine: &MachineKey,
        node: NodeId,
        agent: &HarnessRef,
        anchor: &TriggerAnchor,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.agent_context(Some(machine))
            || super::page(self)
                .and_then(|page| page.inventory())
                .is_none_or(|inventory| inventory.node != node)
        {
            return;
        }
        let Some(bounds) = anchor.get() else {
            return;
        };
        let state = &mut self.areas.settings.agents;
        match Confirmation::new(&state.pages, machine.clone(), agent) {
            Ok(confirmation) => {
                let Some(inventory) = state
                    .pages
                    .get(machine.as_str())
                    .and_then(|page| page.inventory())
                else {
                    return;
                };
                let label = vessel_app_view::agent_status::agent_label(agent, &inventory.agents);
                let title = match self.words.agent_catalog().remove_title(&label) {
                    Ok(title) => title,
                    Err(error) => {
                        tracing::warn!(%error,"provider confirmation words could not be prepared");
                        state.removal.word_error = Some(error);
                        cx.notify();
                        return;
                    }
                };
                let decision = Rc::new(Decision {
                    machine: machine.clone(),
                    node,
                    agent: agent.clone(),
                    label,
                    title: RefCell::new(title.into()),
                    confirmation: RefCell::new(Some(confirmation)),
                });
                state.removal.target = Some(decision);
                state.removal.problem = None;
                state.removal.word_error = None;
                state.removal.menu.open_end(bounds, window, cx);
            }
            Err(problem) => state.removal.problem = Some(problem),
        }
        cx.notify();
    }
    fn agent_choose_removal(
        &mut self,
        origin: &Rc<Decision>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.agent_context(Some(&origin.machine))
            || super::page(self)
                .and_then(|page| page.inventory())
                .is_none_or(|inventory| inventory.node != origin.node)
        {
            return;
        }
        let state = &mut self.areas.settings.agents.removal;
        if !state
            .target
            .as_ref()
            .is_some_and(|target| Rc::ptr_eq(target, origin))
        {
            return;
        }
        state.confirmation = Some(Rc::clone(origin));
        state.dialog.focus(window, cx);
        cx.notify();
    }
    fn agent_cancel_removal(
        &mut self,
        origin: &Rc<Decision>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.settings.agents.removal;
        if state
            .confirmation
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, origin))
        {
            state.confirmation = None;
            state.target = None;
            state.dialog.dismissed(window, cx);
            cx.notify();
        }
    }
    fn agent_confirm_removal(
        &mut self,
        origin: &Rc<Decision>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.agent_context(Some(&origin.machine)) {
            return;
        }
        let state = &mut self.areas.settings.agents;
        if !state
            .removal
            .confirmation
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, origin))
        {
            return;
        }
        let Some(confirmation) = origin.confirmation.borrow_mut().take() else {
            return;
        };
        let result = confirmation.confirm(&origin.machine, &mut state.pages);
        state.removal.confirmation = None;
        state.removal.target = None;
        state.removal.dialog.dismissed(window, cx);
        match result {
            Ok(request) => self.command(vessel_app_link::Command::remove_agent(request)),
            Err(problem) => self.areas.settings.agents.removal.problem = Some(problem),
        }
        cx.notify();
    }
}

// Reword the retained decision at the same language boundary as other dialogs.
// A failed formatter cancels the local confirmation, never the remote machine.
pub(super) fn refresh_words(shell: &mut Shell, _window: &mut Window, cx: &mut Context<'_, Shell>) {
    let words = shell.words.agent_catalog();
    let state = &mut shell.areas.settings.agents.removal;
    let Some(target) = state.target.as_ref() else {
        return;
    };
    match words.remove_title(&target.label) {
        Ok(title) => *target.title.borrow_mut() = title.into(),
        Err(error) => {
            tracing::warn!(%error,"provider confirmation words could not be refreshed");
            state.reset();
            state.word_error = Some(error);
        }
    }
    cx.notify();
}
