//! All input stays scoped to the side-only boundary, including resets.

use super::SIDE;
use crate::routes::Route;
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::side_question::{Action, Ask, Command as SideCommand};
use vessel_app_link::{Act, ActAnswer, Command};
use vessel_app_view::devices::MachineKey;
use vessel_app_view::side_question::{Operation, Question, Target};
use vessel_proto::SessionId;

impl Shell {
    pub(crate) fn side_toggle(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.areas.side_question.state.target().is_some() {
            self.side_close();
            self.areas.session.composer.focus(window, cx);
        } else if let Some(open) = &self.areas.session.open {
            let target = Target {
                machine: MachineKey::from_book(&open.machine),
                parent: open.session,
            };
            self.areas.side_question.composer.clear(window, cx);
            self.areas.workspace.code.open = false;
            self.areas.side_question.state.select(target);
            self.side_open();
            self.areas.side_question.composer.focus(window, cx);
        }
        cx.notify();
    }

    pub(crate) fn side_close(&mut self) {
        self.areas.side_question.state.close();
        self.areas.side_question.follow = None;
        self.areas.side_question.sent = None;
        *self.areas.side_question.acknowledged.borrow_mut() = None;
        self.command(Command::SideQuestion(SideCommand::Close));
    }

    pub(crate) fn side_book_changed(&mut self, machine: &str) {
        if self
            .areas
            .side_question
            .state
            .target()
            .is_some_and(|target| target.machine.as_str() == machine)
        {
            self.side_close();
        }
    }

    pub(crate) fn side_parent_changed(&mut self, machine: &str, parent: SessionId) {
        if self
            .areas
            .side_question
            .state
            .target()
            .is_some_and(|target| target.machine.as_str() != machine || target.parent != parent)
        {
            self.side_close();
        }
    }

    pub(crate) fn side_route_changed(&mut self, route: Route) {
        if route != Route::Session && self.areas.side_question.state.target().is_some() {
            self.side_close();
        }
    }

    pub(crate) fn side_open(&mut self) {
        self.side_run(Operation::Open, Action::Open);
    }

    pub(crate) fn side_reset(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.areas.side_question.composer.focus(window, cx);
        if let Some(side) = self.areas.side_question.state.side() {
            self.areas.side_question.follow = None;
            self.command(Command::SideQuestion(SideCommand::Close));
            self.side_run(Operation::Reset, Action::Reset(side));
            cx.notify();
        }
    }

    pub(crate) fn side_submit(
        &mut self,
        text: String,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.side_question;
        if !area.state.may_prompt() || area.conversation.timeline.in_turn() {
            return;
        }
        let result = Question::parse(&text);
        match result {
            Ok(question) => {
                if let Some(side) = area.state.side() {
                    area.sent = Some(text);
                    self.side_run(Operation::Prompt, Action::Prompt { side, question });
                }
            }
            Err(problem) => area.state.refused(problem),
        }
        cx.notify();
    }

    fn side_run(&mut self, operation: Operation, action: Action) {
        let result = self
            .areas
            .side_question
            .state
            .begin(operation)
            .and_then(|pending| Ask::new(pending, action));
        match result {
            Ok(ask) => self.command(Command::SideQuestion(SideCommand::Run(ask))),
            Err(problem) => self.areas.side_question.state.refused(problem),
        }
    }

    pub(crate) fn side_stop(&self) {
        if let Some(shown) = &self.areas.side_question.conversation.shown {
            self.command(Command::Stop {
                machine: shown.machine.clone(),
                session: shown.session,
            });
        }
    }

    pub(crate) fn side_acted(
        &mut self,
        answer: ActAnswer,
        cx: &mut Context<'_, Self>,
    ) -> Option<ActAnswer> {
        let area = &mut self.areas.side_question;
        if area.state.target().is_none()
            || !area.conversation.shown.as_ref().is_some_and(|shown| {
                shown.machine == answer.machine && shown.session == answer.session
            })
        {
            return Some(answer);
        }
        if area.conversation.answered(answer) == super::super::conversation::Followup::ReadWaiting {
            self.act_at(SIDE, Act::Conversation);
        }
        cx.notify();
        None
    }
}
