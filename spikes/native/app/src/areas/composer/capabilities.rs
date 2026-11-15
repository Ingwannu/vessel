//! Creation can finish in the background; its upload never follows the UI.
use crate::shell::Shell;
use std::collections::HashMap;
use vessel_app_kit::ui::Context;
use vessel_app_link::{
    Command,
    attachments::{CapabilityAsk, CapabilityFailure, Files},
};
use vessel_app_view::{
    composer::{
        drafts::DraftKey,
        tray::{Again, ItemState, Problem, Target},
    },
    creation::Ticket,
    machine_book::Name,
};
use vessel_proto::{NodeId, SessionId};
#[derive(Debug, Clone)]
enum State {
    Reading(CapabilityAsk),
    Ready { ask: CapabilityAsk, target: Target },
    Failed(CapabilityAsk),
}
#[derive(Debug, Default)]
pub(super) struct Capabilities {
    states: HashMap<SessionId, State>,
}
impl Capabilities {
    pub(super) fn target(
        &self,
        session: SessionId,
        connection: &vessel_app_view::connection::ConnectionContext,
    ) -> Option<Target> {
        match self.states.get(&session) {
            Some(State::Ready { ask, target }) if &ask.connection == connection => Some(*target),
            None | Some(State::Reading(_) | State::Failed(_) | State::Ready { .. }) => None,
        }
    }
    pub(super) fn forget(&mut self, session: SessionId) {
        drop(self.states.remove(&session));
    }
    pub(super) fn blocked(&self, session: SessionId) -> bool {
        matches!(
            self.states.get(&session),
            Some(State::Reading(_) | State::Failed(_))
        )
    }
}
impl Shell {
    pub(super) fn attachment_context(
        &self,
        name: &Name,
    ) -> Option<vessel_app_view::connection::ConnectionContext> {
        self.fleet
            .machines
            .iter()
            .find(|machine| machine.name == name.as_str())
            .and_then(vessel_app_view::Machine::connection_context)
    }

    pub(super) fn read_creation_capabilities(
        &mut self,
        ticket: Ticket,
        machine: Name,
        expected_node: NodeId,
        connection: vessel_app_view::connection::ConnectionContext,
        session: SessionId,
    ) -> Result<(), CapabilityFailure> {
        let ask = CapabilityAsk {
            connection,
            ticket,
            machine,
            expected_node,
            session,
        };
        self.areas
            .composer
            .capabilities
            .states
            .retain(|key, _| self.areas.composer.sends.contains_key(key));
        if self.areas.composer.capabilities.states.len()
            >= vessel_app_view::composer::trays::Trays::MAX
            && !self
                .areas
                .composer
                .capabilities
                .states
                .contains_key(&session)
        {
            return Err(CapabilityFailure::Capacity);
        }
        let _previous = self
            .areas
            .composer
            .capabilities
            .states
            .insert(session, State::Reading(ask.clone()));
        self.command(Command::Files(Files::Capabilities(ask)));
        Ok(())
    }
    pub(super) fn fail_creation_preparation(
        &mut self,
        session: SessionId,
        why: &CapabilityFailure,
        cx: &mut Context<'_, Self>,
    ) {
        tracing::info!(?why, %session, "attachment preparation failed");
        let message = self.words.composer().file_capabilities_failed().to_owned();
        if let Some(tray) = self
            .areas
            .composer
            .trays
            .get_mut(DraftKey::Session { session })
        {
            let keys: Vec<_> = tray.items().iter().map(|item| item.key).collect();
            for key in keys {
                tray.failed(key, Problem::Node(message.clone()), Again::Restart);
            }
        }
        cx.notify();
    }
    pub(crate) fn composer_capabilities(
        &mut self,
        ask: &CapabilityAsk,
        result: Result<Target, CapabilityFailure>,
        cx: &mut Context<'_, Self>,
    ) {
        let expected = matches!(self.areas.composer.capabilities.states.get(&ask.session), Some(State::Reading(current)) if current == ask)
            && self
                .areas
                .composer
                .sends
                .get(&ask.session)
                .is_some_and(|waiting| {
                    waiting.creation == Some(ask.ticket)
                        && waiting.expected_node == ask.expected_node
                        && waiting.connection == ask.connection
                });
        if !expected {
            return;
        }
        match result {
            Ok(target) => {
                let _old = self.areas.composer.capabilities.states.insert(
                    ask.session,
                    State::Ready {
                        ask: ask.clone(),
                        target,
                    },
                );
                if let Some(tray) = self.areas.composer.trays.get_mut(DraftKey::Session {
                    session: ask.session,
                }) {
                    tray.target(Some(target));
                    let keys: Vec<_> = tray
                        .items()
                        .iter()
                        .filter(|item| matches!(item.state, ItemState::Failed { .. }))
                        .map(|item| item.key)
                        .collect();
                    for key in keys {
                        let _retried = tray.retry(key);
                    }
                }
                self.composer_pump(ask.session, cx);
            }
            Err(why) => {
                let _old = self
                    .areas
                    .composer
                    .capabilities
                    .states
                    .insert(ask.session, State::Failed(ask.clone()));
                self.fail_creation_preparation(ask.session, &why, cx);
            }
        }
    }
    pub(super) fn retry_creation_capabilities(
        &mut self,
        session: SessionId,
        cx: &mut Context<'_, Self>,
    ) {
        let ask = match self.areas.composer.capabilities.states.get(&session) {
            Some(State::Reading(_) | State::Ready { .. }) => return,
            Some(State::Failed(ask)) => ask.clone(),
            None => {
                let Some(waiting) = self.areas.composer.sends.get(&session) else {
                    return;
                };
                let Some(ticket) = waiting.creation else {
                    return;
                };
                CapabilityAsk {
                    connection: waiting.connection.clone(),
                    machine: waiting.machine.clone(),
                    expected_node: waiting.expected_node,
                    session,
                    ticket,
                }
            }
        };
        if let Err(why) = self.read_creation_capabilities(
            ask.ticket,
            ask.machine,
            ask.expected_node,
            ask.connection,
            session,
        ) {
            self.fail_creation_preparation(session, &why, cx);
        }
    }
}
