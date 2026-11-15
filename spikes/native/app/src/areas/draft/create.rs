//! Correlated completion owns payload settlement independently of navigation.

use crate::{routes::Route, shell::Shell};
use vessel_app_kit::ui::Context;
use vessel_app_link::creation::Answer;
use vessel_app_view::creation::{Completion, Failure, Flow, Navigation, Ticket, Transition};
use vessel_app_view::{Card, Picks};

/// UI payload recovery stays separate from pure lifecycle rules.
pub(crate) struct CreationState {
    pub(crate) flow: Flow,
    pub(crate) registered: Option<Card>,
    pub(crate) node: Option<vessel_proto::NodeId>,
    pub(crate) context: Option<vessel_app_view::connection::ConnectionContext>,
    next: Option<Ticket>,
    input: Option<(Ticket, vessel_proto::timeline::ClientMessageId)>,
}
impl Default for CreationState {
    fn default() -> Self {
        Self {
            flow: Flow::default(),
            registered: None,
            node: None,
            context: None,
            next: Some(Ticket::new(0)),
            input: None,
        }
    }
}
impl CreationState {
    /// A rejected first input retains its identity when retried on the same child.
    pub(super) fn message(&mut self, ticket: Ticket) -> vessel_proto::timeline::ClientMessageId {
        if let Some((current, message)) = &self.input
            && *current == ticket
        {
            return message.clone();
        }
        let message = vessel_proto::timeline::ClientMessageId::random();
        self.input = Some((ticket, message.clone()));
        message
    }
    pub(super) fn allocate(&mut self) -> Option<Ticket> {
        let ticket = self.next?;
        self.next = ticket.checked_next();
        Some(ticket)
    }
}
impl Shell {
    /// Shared registration forwards here; stale replies consume no payload.
    pub(crate) fn creation_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        if answer.source() != vessel_app_view::composer::drafts::DraftKey::New {
            // Workspace drafts use their own keyed owner once that bridge lands.
            return;
        }
        match answer {
            Answer::Registered {
                ticket,
                machine,
                node,
                context,
                session,
                ..
            } => {
                if self.areas.draft.creation.flow.ticket() == Some(ticket) {
                    self.areas.draft.creation.node = Some(node);
                    self.areas.draft.creation.context = Some(context);
                    self.areas.draft.creation.registered =
                        Some(Card::from_session(machine.as_str(), &session));
                }
            }
            Answer::Progress { ticket, stage, .. } => {
                let _transition = self.areas.draft.creation.flow.progress(ticket, stage);
            }
            Answer::Complete(_, ticket) => self.creation_complete(ticket, cx),
            Answer::Failed {
                ticket,
                stage,
                failure,
                ..
            } => {
                let _transition = self.areas.draft.creation.flow.progress(ticket, stage);
                if self.areas.draft.creation.flow.fail(ticket, failure) == Transition::Applied {
                    if failure != Failure::OutcomeUnknown {
                        self.rejected_draft_files(ticket, cx);
                    }
                    self.creation_failed();
                }
            }
        }
        cx.notify();
    }
    fn creation_complete(&mut self, ticket: Ticket, cx: &mut Context<'_, Self>) {
        if self.areas.draft.creation.flow.ticket() != Some(ticket) {
            return;
        }
        if let Some(card) = &self.areas.draft.creation.registered {
            let same_node = self.fleet.machines.iter().any(|machine| {
                machine.name == card.machine
                    && machine.connection_context().as_ref()
                        == self.areas.draft.creation.context.as_ref()
                    && machine.node == self.areas.draft.creation.node
            });
            if !same_node {
                self.creation_answer(
                    Answer::Failed {
                        source: vessel_app_view::composer::drafts::DraftKey::New,
                        ticket,
                        stage: vessel_app_view::creation::Stage::InitialInput,
                        // Completion may already include accepted first input.
                        // A retired admission cannot turn that into a retryable
                        // refusal and send the prompt again on a new connection.
                        failure: Failure::OutcomeUnknown,
                    },
                    cx,
                );
                self.reload();
                return;
            }
        }
        let completion = self.areas.draft.creation.flow.complete(ticket);
        if completion == Completion::Ignored {
            return;
        }
        if let Some(card) = self.areas.draft.creation.registered.take() {
            // Leaving relinquishes navigation, never captured file delivery.
            self.started_draft_files(ticket, &card, cx);
            if completion == Completion::Open {
                self.started(&card, cx);
            }
        }
        self.draft_started();
        *self.areas.draft.restore.borrow_mut() = None;
        self.areas.draft.picks = Picks::default();
        self.areas.draft.chosen = false;
        self.reload();
    }
    fn creation_failed(&mut self) {
        let foreground = matches!(
            self.areas.draft.creation.flow,
            Flow::Failed {
                navigation: Navigation::Foreground,
                ..
            }
        );
        self.areas
            .draft
            .restore
            .borrow_mut()
            .clone_from(&self.areas.draft.sending);
        if foreground {
            self.route_to(Route::Draft);
        }
    }
    /// Lock the actual editor, including input methods and paste, while held.
    pub(super) fn creation_editor(&self, cx: &mut Context<'_, Self>) {
        let editability = if self.areas.draft.creation.flow.may_submit() {
            vessel_app_kit::Editability::Editable
        } else {
            vessel_app_kit::Editability::ReadOnly
        };
        self.areas.draft.composer.set_editability(editability, cx);
    }
    /// Navigation relinquishes this submission's route ownership permanently.
    pub(crate) fn creation_departing(&mut self) {
        self.areas.draft.creation.flow.leave();
    }
    /// A created session consumed the original draft's pending text.
    pub(crate) fn draft_started(&mut self) {
        self.areas.draft.sending = None;
    }
    /// Legacy callers retain their existing restoration behavior.
    pub(crate) fn start_failed(&mut self) {
        if self.areas.draft.sending.is_some() {
            self.route_to(Route::Draft);
            let text = self.areas.draft.sending.take();
            *self.areas.draft.restore.borrow_mut() = text;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn queued_completion_after_same_node_reconnect_retains_input_without_retry(
        cx: &mut gpui::TestAppContext,
    ) {
        crate::shell::language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, _window, cx| {
            use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
            let node = vessel_proto::NodeId::random();
            let mut admissions = ConnectionEpochs::default();
            let original = admissions
                .admit(MachineKey::from_book("fixture"), node)
                .unwrap();
            let replacement = admissions
                .admit(MachineKey::from_book("fixture"), node)
                .unwrap();
            let mut machine = vessel_app_view::Machine::unreachable("fixture".into(), "fixture");
            machine.node = Some(node);
            machine.connection_epoch = Some(replacement.epoch());
            shell.fleet.machines.push(machine);
            let id = vessel_proto::SessionId::random();
            let info = vessel_proto::SessionInfo::new(
                id,
                vessel_proto::SessionSpec::new(
                    vessel_proto::HarnessKind::Codex,
                    vessel_proto::SessionMode::Structured {
                        model: None,
                        resume: None,
                        options: Vec::new(),
                    },
                ),
                chrono::DateTime::UNIX_EPOCH,
            );
            let ticket = Ticket::new(1);
            let creation = &mut shell.areas.draft.creation;
            creation.node = Some(node);
            creation.context = Some(original);
            creation.registered = Some(Card::from_session("fixture", &info));
            assert_eq!(
                creation
                    .flow
                    .begin(ticket, vessel_app_view::creation::Stage::InitialInput),
                Transition::Applied
            );
            shell.areas.draft.sending = Some("retain first input".to_owned());
            shell.creation_complete(ticket, cx);
            assert!(!shell.areas.draft.creation.flow.may_submit());
            assert!(matches!(
                shell.areas.draft.creation.flow,
                Flow::Failed {
                    failure: Failure::OutcomeUnknown,
                    ..
                }
            ));
            assert_eq!(
                shell
                    .areas
                    .draft
                    .creation
                    .registered
                    .as_ref()
                    .unwrap()
                    .session,
                id
            );
            assert_eq!(
                shell.areas.draft.sending.as_deref(),
                Some("retain first input")
            );
            assert!(shell.areas.session.open.is_none());
        });
    }

    #[gpui::test]
    fn another_drafts_same_local_ticket_cannot_complete_the_top_level_draft(
        cx: &mut gpui::TestAppContext,
    ) {
        crate::shell::language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, _window, cx| {
            let ticket = Ticket::new(7);
            assert_eq!(
                shell
                    .areas
                    .draft
                    .creation
                    .flow
                    .begin(ticket, vessel_app_view::creation::Stage::Resource),
                Transition::Applied
            );
            let original = shell.areas.draft.creation.flow.clone();
            shell.creation_answer(
                Answer::Complete(
                    vessel_app_view::composer::drafts::DraftKey::Session {
                        session: vessel_proto::SessionId::random(),
                    },
                    ticket,
                ),
                cx,
            );
            assert_eq!(shell.areas.draft.creation.flow, original);
        });
    }

    #[test]
    fn first_input_identity_survives_preparation_and_same_child_retry() {
        let mut state = CreationState::default();
        let first = state.allocate().unwrap();
        let message = state.message(first);
        assert_eq!(state.message(first), message);
        let next = state.allocate().unwrap();
        assert_ne!(state.message(next), message);
    }
}
