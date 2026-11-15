//! Manager intents use the shell's existing bounded queue/refusal settlement.
//! No draft or catalog changes optimistically; results retain host and ticket.

use vessel_app_kit::ui::Context;
use vessel_app_link::labels::{Answer, Command, Event, Request};
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::labels::{Failure, HostKey, Manager, ManagerState, Name, Ticket};

use super::Labels;
use crate::shell::Shell;

#[derive(Clone)]
pub(super) struct Target {
    opening: super::surface::Opening,
    host: HostKey,
    connection: Option<ConnectionContext>,
    name: Name,
}

impl Target {
    pub(super) const fn new(
        opening: super::surface::Opening,
        host: HostKey,
        connection: Option<ConnectionContext>,
        name: Name,
    ) -> Self {
        Self {
            opening,
            host,
            connection,
            name,
        }
    }
    pub(super) const fn opening(&self) -> super::surface::Opening {
        self.opening
    }
    pub(super) fn matches(
        &self,
        opening: Option<super::surface::Opening>,
        manager: &Manager,
        connection: Option<&ConnectionContext>,
    ) -> bool {
        opening == Some(self.opening)
            && self.connection.as_ref() == connection
            && manager.selected() == Some(&self.host)
            && manager
                .state()
                .draft()
                .is_some_and(|draft| draft.original().name.key() == self.name.key())
    }
}

#[derive(Clone)]
pub(super) enum Action {
    Save(Target),
    Inspect(Target),
    Delete { target: Target, inspection: Ticket },
}

impl Action {
    fn is_current(
        &self,
        opening: Option<super::surface::Opening>,
        manager: &Manager,
        connection: Option<&ConnectionContext>,
    ) -> bool {
        let target = match self {
            Self::Save(target) | Self::Inspect(target) | Self::Delete { target, .. } => target,
        };
        target.matches(opening, manager, connection)
            && match self {
                Self::Save(_) | Self::Inspect(_) => {
                    matches!(manager.state(), ManagerState::Editing(_))
                }
                Self::Delete { inspection, .. } => {
                    matches!(manager.state(), ManagerState::Confirming { ticket, .. } if ticket == inspection)
                }
            }
    }
}

impl Labels {
    fn prepare(&mut self, action: &Action) -> Result<Request, Failure> {
        let host = self.selected_host().ok_or(Failure::Offline)?;
        let context = host.connection.clone().ok_or(Failure::Changed)?;
        let state = host.state;
        let catalog = host.catalog.clone();
        state.writable()?;
        let ticket = self.tickets.issue()?;
        let ask = match action {
            Action::Save(_) => {
                let original = self
                    .manager
                    .state()
                    .draft()
                    .ok_or(Failure::NoChange)?
                    .original()
                    .name
                    .clone();
                let replacement = self.manager.save(state, &catalog, ticket)?;
                Command::Update {
                    original,
                    replacement,
                }
            }
            Action::Inspect(_) => {
                Command::InspectDelete(self.manager.inspect(state, &catalog, ticket)?)
            }
            Action::Delete { .. } => Command::Delete(self.manager.delete(state, ticket)?),
        };
        self.error = None;
        self.pending = Some((context.clone(), ticket));
        Ok(Request {
            context,
            ticket,
            ask,
        })
    }
}

impl Shell {
    pub(super) fn label_action(&mut self, action: &Action, cx: &mut Context<'_, Self>) {
        let state = &self.areas.labels;
        if !action.is_current(
            state.opening,
            &state.manager,
            state
                .selected_host()
                .and_then(|host| host.connection.as_ref()),
        ) {
            return;
        }
        match self.areas.labels.prepare(action) {
            Ok(Request {
                context,
                ticket,
                ask,
            }) => {
                self.command(vessel_app_link::Command::Labels(
                    vessel_app_link::labels::Request {
                        context,
                        ticket,
                        ask,
                    },
                ));
            }
            Err(failure) => self.areas.labels.error = Some(failure),
        }
        self.settle_labels(cx);
        cx.notify();
    }

    pub(crate) fn labels_answer(&mut self, event: Event, cx: &mut Context<'_, Self>) {
        #[cfg(test)]
        if self.labels_picker_answer(&event, cx) {
            return;
        }
        let state = &mut self.areas.labels;
        if state.pending.as_ref() != Some(&(event.context.clone(), event.ticket)) {
            return;
        }
        let inspection = matches!(&event.answer, Answer::Inspected(Ok(_)));
        let accepted = match event.answer {
            Answer::Listed(_) | Answer::Assigned { .. } => false,
            Answer::Inspected(result) => state.manager.inspected(event.ticket, result),
            Answer::Updated(result) => {
                matches!(state.manager.state(), ManagerState::Saving { .. })
                    && state.manager.completed(event.ticket, result.map(|_| ()))
            }
            Answer::Deleted(result) => {
                matches!(state.manager.state(), ManagerState::Deleting { .. })
                    && state.manager.completed(event.ticket, result.map(|_| ()))
            }
        };
        if accepted {
            state.pending = None;
            if inspection {
                state.confirmation = Some(event.context);
            }
        }
        self.settle_labels(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::areas::labels::surface::Opening;
    use vessel_app_view::labels::{Catalog, Color, Count, HostState, Label, Tickets};

    fn label(name: &str) -> Label {
        Label {
            name: name.parse().unwrap(),
            color: Color::Violet,
        }
    }

    #[test]
    fn labels_rendered_actions_cannot_retarget_a_new_editor_host_or_opening() {
        let mut tickets = Tickets::default();
        let opening = Opening::new(tickets.issue().unwrap());
        let host = HostKey::random();
        let mut manager = Manager::default();
        manager.select(host).unwrap();
        manager.edit(label("Ready")).unwrap();
        let action = Action::Save(Target::new(opening, host, None, "Ready".parse().unwrap()));
        assert!(action.is_current(Some(opening), &manager, None));
        assert!(!action.is_current(Some(Opening::new(tickets.issue().unwrap())), &manager, None));
        manager.edit(label("Other")).unwrap();
        assert!(!action.is_current(Some(opening), &manager, None));
        manager.select(HostKey::random()).unwrap();
        manager.edit(label("Ready")).unwrap();
        assert!(!action.is_current(Some(opening), &manager, None));
    }

    #[test]
    fn labels_old_confirmation_action_cannot_consume_a_new_inspection() {
        let mut tickets = Tickets::default();
        let opening = Opening::new(tickets.issue().unwrap());
        let host = HostKey::random();
        let mut manager = Manager::default();
        manager.select(host).unwrap();
        manager.edit(label("Ready")).unwrap();
        let target = Target::new(opening, host, None, "Ready".parse().unwrap());
        let inspect_action = Action::Inspect(target.clone());
        let catalog = Catalog::snapshot([label("Ready")]);
        let first = tickets.issue().unwrap();
        let _target = manager.inspect(HostState::Online, &catalog, first).unwrap();
        assert!(!inspect_action.is_current(Some(opening), &manager, None));
        assert!(manager.inspected(first, Ok(Count::new(1))));
        let old = Action::Delete {
            target: target.clone(),
            inspection: first,
        };
        assert!(old.is_current(Some(opening), &manager, None));
        assert!(manager.cancel_delete());
        assert!(!old.is_current(Some(opening), &manager, None));
        let next = tickets.issue().unwrap();
        let _target = manager.inspect(HostState::Online, &catalog, next).unwrap();
        assert!(manager.inspected(next, Ok(Count::new(2))));
        assert!(!old.is_current(Some(opening), &manager, None));
        assert!(
            Action::Delete {
                target,
                inspection: next
            }
            .is_current(Some(opening), &manager, None)
        );
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    use vessel_app_view::{
        connection::{ConnectionEpochs, MachineKey},
        labels::{Color, Label, Tickets},
    };

    #[test]
    fn labels_callback_rejects_reconnect_or_another_alias_even_for_the_same_node() {
        let mut epochs = ConnectionEpochs::default();
        let node = HostKey::random();
        let original = epochs.admit(MachineKey::from_book("first"), node).unwrap();
        let reconnect = epochs.admit(MachineKey::from_book("first"), node).unwrap();
        let other = epochs.admit(MachineKey::from_book("other"), node).unwrap();
        let opening = super::super::surface::Opening::new(Tickets::default().issue().unwrap());
        let name: Name = "Ready".parse().unwrap();
        let mut manager = Manager::default();
        manager.select(node).unwrap();
        manager
            .edit(Label {
                name: name.clone(),
                color: Color::Violet,
            })
            .unwrap();
        let action = Action::Save(Target::new(opening, node, Some(original.clone()), name));
        assert!(action.is_current(Some(opening), &manager, Some(&original)));
        assert!(!action.is_current(Some(opening), &manager, Some(&reconnect)));
        assert!(!action.is_current(Some(opening), &manager, Some(&other)));
        assert!(!action.is_current(Some(opening), &manager, None));
    }
}

#[cfg(test)]
mod dispatch_tests;
