//! Host orchestration UI uses one pure owner model and existing kit controls.
mod draw;
mod modal;
mod retained;
#[cfg(test)]
mod tests;
pub(crate) use draw::render;
pub(crate) use modal::dialog;

use crate::{routes::Route, shell::Shell};
use vessel_app_kit::dialog::DialogState;
use vessel_app_kit::ui::{App, ClickEvent, Context, Window};
use vessel_app_link::Command;
use vessel_app_view::devices::{Ticket, Tickets};
use vessel_app_view::orchestration::{Answer, Consumer, Page, Problem, Scope, Task, Visit};
use vessel_app_view::settings::Section;
use vessel_proto::skills::SkillName;

/// The controller keeps ticket allocation across all visits, including A-B-A.
pub(crate) struct State {
    pub(super) page: Option<Page>,
    pub(super) exhausted: bool,
    pub(super) dialog: DialogState,
    dialog_ticket: Option<Ticket>,
    tickets: Tickets,
    closed: bool,
    retired: Option<vessel_app_view::connection::ConnectionTag>,
    retained_draft: Option<retained::Draft>,
    activation: Option<gpui::Subscription>,
    pub(super) turns: u64,
}
impl State {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            page: None,
            exhausted: false,
            dialog: DialogState::new(cx),
            dialog_ticket: None,
            tickets: Tickets::default(),
            closed: false,
            retired: None,
            retained_draft: None,
            activation: None,
            turns: 0,
        }
    }
    fn retain_draft(&mut self) {
        if let Some(draft) = self.page.as_ref().and_then(retained::Draft::capture) {
            self.retained_draft = Some(draft);
        }
    }
    fn ticket(&mut self) -> Option<Ticket> {
        match self.tickets.next_ticket() {
            Ok(ticket) => Some(ticket),
            Err(_exhausted) => {
                self.exhausted = true;
                None
            }
        }
    }
    fn open(&self) -> bool {
        self.page
            .as_ref()
            .is_some_and(|page| page.picker.is_some() || page.confirmation.is_some())
    }
    fn renew_dialog(&mut self) {
        if self.open() {
            if let Some(ticket) = self.ticket() {
                self.dialog_ticket = Some(ticket);
            }
        } else {
            self.dialog_ticket = None;
        }
    }
    fn settle_focus(&mut self, window: &mut Window, cx: &mut Context<'_, Shell>) {
        if self.closed {
            self.dialog.dismissed(window, cx);
            self.closed = false;
        }
    }
}
/// UI intent, never an RPC name or arbitrary configuration patch.
#[derive(Clone)]
pub(super) enum Intent {
    Refresh,
    Tools,
    Skills,
    Update,
    Choose,
    All,
    Toggle(SkillName),
    Save,
    Confirm,
    Cancel,
}

impl Shell {
    /// Enter/reenter reads once; leaving or a changed welcome identity drops scope.
    pub(crate) fn settle_orchestration(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.areas.settings.orchestration.activation.is_none() {
            self.areas.settings.orchestration.activation =
                Some(cx.observe_window_activation(window, |shell, window, cx| {
                    if window.is_window_active()
                        && shell.areas.settings.orchestration.page.is_some()
                    {
                        shell.orchestration_refresh(cx);
                        cx.notify();
                    }
                }));
        }
        let shown = self.route == Route::Settings
            && self.areas.settings.section == Section::Overview
            && (!vessel_app_kit::breakpoint::phone(window) || self.areas.settings.pushed);
        let target = super::choices::chosen_machine(self)
            .filter(|_machine| shown)
            .and_then(vessel_app_view::Machine::connection_context)
            .filter(|context| {
                self.areas.settings.orchestration.retired.as_ref() != Some(&context.tag())
            });
        let state = &mut self.areas.settings.orchestration;
        state.settle_focus(window, cx);
        let Some(target) = target else {
            if shown {
                state.retain_draft();
            } else {
                state.retained_draft = None;
            }
            state.closed = state.open();
            state.page = None;
            state.settle_focus(window, cx);
            return;
        };
        if state
            .page
            .as_ref()
            .is_some_and(|page| page.scope().target == target)
        {
            return;
        }
        state.retain_draft();
        state.retained_draft = state
            .retained_draft
            .take()
            .filter(|draft| draft.matches(&target));
        state.closed = state.open();
        state.page = None;
        state.settle_focus(window, cx);
        let Some(visit) = state.ticket().map(Visit::new) else {
            return;
        };
        state.page = Some(Page::new(Scope {
            consumer: Consumer::Settings,
            target,
            visit,
        }));
        self.orchestration_refresh(cx);
    }
    /// A retired admission cannot be revived by an older queued Fleet snapshot.
    pub(crate) fn orchestration_retired(
        &mut self,
        tag: &vessel_app_view::connection::ConnectionTag,
    ) -> bool {
        let state = &mut self.areas.settings.orchestration;
        if !state
            .page
            .as_ref()
            .is_some_and(|page| page.scope().target.tag() == *tag)
        {
            return false;
        }
        state.retain_draft();
        state.closed |= state.open();
        state.page = None;
        state.retired = Some(tag.clone());
        true
    }
    fn orchestration_refresh(&mut self, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.settings.orchestration;
        let (tools, skills) = (state.ticket(), state.ticket());
        let Some(page) = state.page.as_mut() else {
            return;
        };
        let tasks = [
            tools.and_then(|ticket| page.read_tools(ticket)),
            skills.and_then(|ticket| page.read_skills(ticket)),
        ];
        for task in tasks.into_iter().flatten() {
            self.orchestration_send(task, cx);
        }
    }
    fn orchestration_send(&mut self, task: Task, _cx: &mut Context<'_, Self>) {
        // Unsupported features are not invoked, even for a read. The link
        // repeats this check against its actual negotiated connection.
        let supported = vessel_proto::mcp::MCP_PARITY.is_some_and(|required| {
            self.fleet.machines.iter().any(|machine| {
                machine.connection_context().as_ref() == Some(&task.scope.target)
                    && machine
                        .protocol
                        .is_some_and(|protocol| protocol >= required)
            })
        });
        if supported {
            self.command(Command::Orchestration(task));
        } else {
            let _unclaimed = self.orchestration_answer(task.refused(Problem::Unsupported));
        }
    }
    /// Onboarding retains its own visit/attempt and consumes its tagged answer.
    pub(crate) fn orchestration_answer(&mut self, answer: Answer) -> Option<Answer> {
        match answer.task.scope.consumer {
            Consumer::Onboarding => Some(answer),
            Consumer::Settings => {
                let state = &mut self.areas.settings.orchestration;
                let refreshed = matches!(
                    (&answer.task.action, &answer.result),
                    (
                        vessel_app_view::orchestration::Action::ReadSkills,
                        Ok(vessel_app_view::orchestration::Output::Skills(_))
                    )
                );
                let opened = state.open();
                let accepted = state
                    .page
                    .as_mut()
                    .is_some_and(|page| page.answered(answer));
                if accepted {
                    if refreshed
                        && let Some(draft) = state.retained_draft.take()
                        && let Some(page) = &mut state.page
                    {
                        let _restored = draft.restore(page);
                    }
                    state.renew_dialog();
                }
                state.closed |= opened && !state.open();
                None
            }
        }
    }
    fn orchestration_intent(
        &mut self,
        scope: &Scope,
        intent: Intent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self
            .areas
            .settings
            .orchestration
            .page
            .as_ref()
            .is_some_and(|page| page.scope() == scope)
        {
            return;
        }
        if matches!(intent, Intent::Refresh) {
            self.orchestration_refresh(cx);
            cx.notify();
            return;
        }
        let state = &mut self.areas.settings.orchestration;
        state.settle_focus(window, cx);
        let needs_ticket = matches!(
            intent,
            Intent::Tools | Intent::Skills | Intent::Save | Intent::Confirm
        );
        if state.exhausted && !matches!(intent, Intent::Cancel) {
            return;
        }
        let before = state.open();
        let ticket = if needs_ticket { state.ticket() } else { None };
        let Some(page) = state.page.as_mut() else {
            return;
        };
        let task = match intent {
            Intent::Refresh => None,
            Intent::Tools => ticket.and_then(|ticket| page.toggle_tools(ticket)),
            Intent::Skills => ticket.and_then(|ticket| page.toggle_skills(ticket)),
            Intent::Update => {
                page.ask_update();
                None
            }
            Intent::Choose => {
                page.choose();
                None
            }
            Intent::All => {
                if !page.skills.pending() && page.confirmation.is_none() {
                    if let Some(picker) = &mut page.picker {
                        picker.toggle_all();
                    }
                }
                None
            }
            Intent::Toggle(name) => {
                if !page.skills.pending() && page.confirmation.is_none() {
                    if let Some(picker) = &mut page.picker {
                        picker.toggle(&name);
                    }
                }
                None
            }
            Intent::Save => ticket.and_then(|ticket| page.save_picker(ticket)),
            Intent::Confirm => ticket.and_then(|ticket| page.confirm(ticket)),
            Intent::Cancel => {
                if !page.skills.pending() {
                    if page.confirmation.is_some() {
                        page.cancel_confirmation();
                    } else {
                        page.close_picker();
                    }
                }
                None
            }
        };
        let after = state.open();
        state.renew_dialog();
        if !before && after {
            state.dialog.focus(window, cx);
        }
        state.closed |= before && !after;
        state.settle_focus(window, cx);
        state.turns = state.turns.saturating_add(1);
        if let Some(task) = task {
            self.orchestration_send(task, cx);
        }
        cx.notify();
    }
}

/// Every drawn action captures its original visit, including dismissal.
pub(super) fn on(
    scope: &Scope,
    intent: Intent,
    cx: &Context<'_, Shell>,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static + use<> {
    let scope = scope.clone();
    cx.listener(move |shell, _event, window, cx| {
        if !shell.areas.settings.orchestration.open() {
            shell.orchestration_intent(&scope, intent.clone(), window, cx);
        }
    })
}

/// Distinguish closing/reopening or replacing decisions within the same visit.
#[derive(Clone)]
pub(super) struct DialogScope {
    scope: Scope,
    ticket: Ticket,
}
impl Shell {
    fn orchestration_dialog_intent(
        &mut self,
        captured: &DialogScope,
        intent: Intent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.settings.orchestration.dialog_ticket == Some(captured.ticket) {
            self.orchestration_intent(&captured.scope, intent, window, cx);
        }
    }
}
pub(super) fn on_dialog(
    captured: &DialogScope,
    intent: Intent,
    cx: &Context<'_, Shell>,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static + use<> {
    let captured = captured.clone();
    cx.listener(move |shell, _event, window, cx| {
        shell.orchestration_dialog_intent(&captured, intent.clone(), window, cx);
    })
}
