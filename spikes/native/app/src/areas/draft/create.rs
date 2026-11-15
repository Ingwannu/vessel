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
    pub(crate) workspace: Option<vessel_app_view::creation::WorkspaceForm>,
    pub(crate) plan: Option<vessel_app_view::creation::LaunchPlan>,
    next: Option<Ticket>,
}
impl Default for CreationState {
    fn default() -> Self {
        Self {
            flow: Flow::default(),
            registered: None,
            node: None,
            workspace: None,
            plan: None,
            next: Some(Ticket::new(0)),
        }
    }
}
impl CreationState {
    pub(super) fn allocate(&mut self) -> Option<Ticket> {
        let ticket = self.next?;
        self.next = ticket.checked_next();
        Some(ticket)
    }
}
impl Shell {
    /// Shared registration forwards here; stale replies consume no payload.
    pub(crate) fn creation_answer(&mut self, answer: Answer, cx: &mut Context<'_, Self>) {
        match answer {
            Answer::Registered {
                ticket,
                machine,
                node,
                session,
            } => {
                if self.areas.draft.creation.flow.ticket() == Some(ticket) {
                    self.areas.draft.creation.node = Some(node);
                    self.areas.draft.creation.registered =
                        Some(Card::from_session(machine.as_str(), &session));
                }
            }
            Answer::Progress { ticket, stage } => {
                let _transition = self.areas.draft.creation.flow.progress(ticket, stage);
            }
            Answer::Complete(ticket) => self.creation_complete(ticket, cx),
            Answer::Failed {
                ticket,
                stage,
                failure,
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
                machine.name == card.machine && machine.node == self.areas.draft.creation.node
            });
            if !same_node {
                self.creation_answer(
                    Answer::Failed {
                        ticket,
                        stage: vessel_app_view::creation::Stage::InitialInput,
                        failure: Failure::Refused(vessel_proto::ErrorCode::Conflict),
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
        *self.areas.draft.restore.borrow_mut() = self.areas.draft.sending.clone();
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
