//! A submission captures choices and locks its ticket before dispatch.

use crate::{areas::composer::PrepareProblem, routes::Route, shell::Shell};
use std::sync::Arc;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::{
    Command,
    creation::{Ask, FirstInput, Target},
};
use vessel_app_view::{
    agent_name, choices_for,
    creation::{Flow, Stage, Ticket, Transition},
    machine_book::Name,
};
use vessel_proto::{HarnessRef, SessionMode, SessionSpec};

impl Shell {
    pub(crate) fn send_draft(
        &mut self,
        text: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.areas.draft.creation.flow.may_submit()
            || self.composer_attachments_blocked(vessel_app_view::composer::drafts::DraftKey::New)
        {
            return;
        }
        if (text.trim().is_empty() && !self.draft_has_files())
            || !self.areas.draft.creation.flow.may_submit()
        {
            return;
        }
        self.settle_draft();
        let Some((machine, harness)) = self.draft_selection() else {
            cx.notify();
            return;
        };
        let Some(context) = self
            .fleet
            .machines
            .iter()
            .find(|entry| entry.name == machine.as_str())
            .and_then(vessel_app_view::Machine::connection_context)
        else {
            self.areas.draft.error = Some(self.words.creation().offline().to_owned());
            cx.notify();
            return;
        };
        let expected_node = context.node();
        let Some((ticket, target)) = self.draft_creation_target(&machine, &context, &harness)
        else {
            cx.notify();
            return;
        };
        let message = self.areas.draft.creation.message(ticket);
        let Some(input) = self.prepare_creation_input(ticket, message.clone(), &context, &text, cx)
        else {
            return;
        };
        let stage = if matches!(target, Target::Registered(_)) {
            Stage::InitialInput
        } else if self.chosen_place().is_some() {
            Stage::Workspace
        } else {
            Stage::Resource
        };
        let transition = if matches!(target, Target::Registered(_)) {
            self.areas.draft.creation.flow.retry_input(ticket)
        } else {
            self.areas.draft.creation.flow.begin(ticket, stage)
        };
        if transition != Transition::Applied {
            self.rejected_draft_files(ticket, cx);
            return;
        }
        let ask = Ask {
            source: vessel_app_view::composer::drafts::DraftKey::New,
            context,
            ticket,
            message,
            machine: machine.clone(),
            expected_node,
            target,
            input,
        };
        self.dispatch_creation(ask, &harness, text, window, cx);
    }

    fn dispatch_creation(
        &mut self,
        ask: Ask,
        harness: &HarnessRef,
        text: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.draft.creation.node = Some(ask.expected_node);
        self.areas.draft.error = None;
        self.areas.draft.sending = Some(text.clone());
        self.areas.draft.composer.clear(window, cx);
        self.begin_pending(
            ask.machine.as_str().to_owned(),
            agent_name(harness),
            text,
            cx,
        );
        self.brand.sent();
        self.route = Route::Session;
        let ask = if matches!(ask.target, Target::New(_)) {
            self.start_in_place(ask)
        } else {
            Some(ask)
        };
        if let Some(ask) = ask {
            self.command(Command::Create(ask));
        }
        cx.notify();
    }

    fn prepare_creation_input(
        &mut self,
        ticket: Ticket,
        message: vessel_proto::timeline::ClientMessageId,
        connection: &vessel_app_view::connection::ConnectionContext,
        text: &str,
        cx: &mut Context<'_, Self>,
    ) -> Option<FirstInput> {
        match self.prepare_draft_input_on(ticket, message, connection.clone(), text) {
            Ok(Some(prepared)) => {
                if prepared.ticket() == ticket {
                    Some(FirstInput::Deferred)
                } else {
                    self.rejected_draft_files(prepared.ticket(), cx);
                    None
                }
            }
            Ok(None) => Some(FirstInput::Text(Arc::from(text))),
            Err(problem) => {
                self.areas.draft.error = Some(
                    match problem {
                        PrepareProblem::Busy | PrepareProblem::Capacity => {
                            self.words.creation().queue_full()
                        }
                        PrepareProblem::Machine(_)
                        | PrepareProblem::Target(_)
                        | PrepareProblem::Context(_) => self.words.creation().refused(),
                    }
                    .to_owned(),
                );
                cx.notify();
                None
            }
        }
    }
    fn draft_selection(&mut self) -> Option<(Name, HarnessRef)> {
        let (Some(machine), Some(harness)) = (
            self.areas.draft.machine.clone(),
            self.areas.draft.agent.clone(),
        ) else {
            self.areas.draft.error = Some(self.t.choose_first.to_owned());
            return None;
        };
        if let Some(why) = self.place_blocked() {
            self.areas.draft.error = Some(why);
            return None;
        }
        if let Ok(machine) = Name::parse(&machine) {
            Some((machine, harness))
        } else {
            self.areas.draft.error = Some(self.words.creation().refused().to_owned());
            None
        }
    }
    fn draft_creation_target(
        &mut self,
        machine: &Name,
        context: &vessel_app_view::connection::ConnectionContext,
        harness: &HarnessRef,
    ) -> Option<(Ticket, Target)> {
        if let Flow::Failed {
            ticket,
            stage: Stage::InitialInput,
            ..
        } = self.areas.draft.creation.flow
            && let Some(card) = &self.areas.draft.creation.registered
        {
            // A rejected first message never makes another agent.
            if card.machine != machine.as_str()
                || self.areas.draft.creation.node != Some(context.node())
                || self.areas.draft.creation.context.as_ref() != Some(context)
            {
                self.areas.draft.error = Some(self.words.creation().refused().to_owned());
                return None;
            }
            return Some((ticket, Target::Registered(card.session)));
        }
        let options = self
            .offer_options(machine.as_str(), harness)
            .map(|offered| choices_for(offered, &self.effective_picks()))
            .unwrap_or_default();
        self.prefs.keep_choices(machine.as_str(), harness, &options);
        let folder = self.areas.draft.folder.trim().to_owned();
        self.prefs.machine = Some(machine.as_str().to_owned());
        self.prefs.agent = Some(harness.clone());
        if folder.is_empty() {
            let _earlier = self.prefs.folders.remove(machine.as_str());
        } else {
            let _earlier = self
                .prefs
                .folders
                .insert(machine.as_str().to_owned(), folder.clone());
        }
        self.command(Command::SavePrefs(Box::new(self.prefs.clone())));
        let mut spec = SessionSpec::new(
            harness.clone(),
            SessionMode::Structured {
                model: None,
                resume: None,
                options,
            },
        );
        spec.cwd = (!folder.is_empty()).then(|| folder.into());
        self.areas.draft.creation.registered = None;
        self.areas
            .draft
            .creation
            .allocate()
            .map(|ticket| (ticket, Target::New(spec)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::language_tests;
    use gpui::TestAppContext;
    use vessel_app_view::Card;
    use vessel_app_view::creation::Failure;
    use vessel_proto::{HarnessKind, NodeId, SessionId, SessionInfo};

    #[gpui::test]
    fn first_input_retry_retains_original_admission_and_cannot_follow_a_reused_alias(
        cx: &mut TestAppContext,
    ) {
        language_tests::init(cx);
        let (shell, cx) = cx.add_window_view(Shell::without_io);
        shell.update_in(cx, |shell, _window, _cx| {
            let machine = Name::parse("fixture").unwrap();
            let harness = HarnessRef::from(HarnessKind::Codex);
            let node = NodeId::random();
            let mut admissions = vessel_app_view::connection::ConnectionEpochs::default();
            let context = admissions
                .admit(
                    vessel_app_view::connection::MachineKey::from_book("fixture"),
                    node,
                )
                .unwrap();
            let session = SessionId::random();
            let ticket = Ticket::new(1);
            let spec = SessionSpec::new(
                harness.clone(),
                SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: Vec::new(),
                },
            );
            let card = Card::from_session(
                machine.as_str(),
                &SessionInfo::new(session, spec, chrono::DateTime::UNIX_EPOCH),
            );
            let creation = &mut shell.areas.draft.creation;
            creation.node = Some(node);
            creation.context = Some(context.clone());
            creation.registered = Some(card);
            assert_eq!(
                creation.flow.begin(ticket, Stage::Resource),
                Transition::Applied
            );
            assert_eq!(
                creation.flow.progress(ticket, Stage::InitialInput),
                Transition::Applied
            );
            assert_eq!(
                creation
                    .flow
                    .fail(ticket, Failure::Refused(vessel_proto::ErrorCode::Conflict)),
                Transition::Applied
            );
            let message = creation.message(ticket);
            let replaced = admissions
                .admit(
                    vessel_app_view::connection::MachineKey::from_book("fixture"),
                    NodeId::random(),
                )
                .unwrap();
            assert!(
                shell
                    .draft_creation_target(&machine, &replaced, &harness)
                    .is_none()
            );
            let reconnected = admissions
                .admit(
                    vessel_app_view::connection::MachineKey::from_book("fixture"),
                    node,
                )
                .unwrap();
            assert!(
                shell
                    .draft_creation_target(&machine, &reconnected, &harness)
                    .is_none()
            );
            assert_eq!(shell.areas.draft.creation.node, Some(node));
            assert_eq!(
                shell
                    .areas
                    .draft
                    .creation
                    .registered
                    .as_ref()
                    .unwrap()
                    .session,
                session
            );
            let (same, target) = shell
                .draft_creation_target(&machine, &context, &harness)
                .unwrap();
            assert_eq!(same, ticket);
            assert!(matches!(target, Target::Registered(id) if id == session));
            assert_eq!(shell.areas.draft.creation.message(ticket), message);
        });
    }
}
