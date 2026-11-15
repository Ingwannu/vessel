//! Manager intents use the shell's existing bounded queue/refusal settlement.
//! No draft or catalog changes optimistically; results retain host and ticket.

use vessel_app_kit::ui::Context;
use vessel_app_link::labels::{Answer, Command, Event};
use vessel_app_view::labels::{Failure, HostKey, Manager, ManagerState, Name, Ticket};

use super::Labels;
use crate::shell::Shell;

#[derive(Clone)]
pub(super) struct Target {
    opening: super::surface::Opening,
    host: HostKey,
    name: Name,
}

impl Target {
    pub(super) const fn new(opening: super::surface::Opening, host: HostKey, name: Name) -> Self {
        Self {
            opening,
            host,
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
    ) -> bool {
        opening == Some(self.opening)
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
    fn is_current(&self, opening: Option<super::surface::Opening>, manager: &Manager) -> bool {
        let target = match self {
            Self::Save(target) | Self::Inspect(target) | Self::Delete { target, .. } => target,
        };
        target.matches(opening, manager)
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
    fn prepare(&mut self, action: Action) -> Result<(HostKey, Ticket, Command), Failure> {
        let host = self
            .manager
            .selected()
            .and_then(|key| self.hosts.iter().find(|host| &host.key == key))
            .ok_or(Failure::Offline)?;
        host.state.writable()?;
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
                let replacement = self.manager.save(host.state, &host.catalog, ticket)?;
                Command::Update {
                    original,
                    replacement,
                }
            }
            Action::Inspect(_) => {
                Command::InspectDelete(self.manager.inspect(host.state, &host.catalog, ticket)?)
            }
            Action::Delete { .. } => Command::Delete(self.manager.delete(host.state, ticket)?),
        };
        self.error = None;
        Ok((host.key, ticket, ask))
    }
}

impl Shell {
    pub(super) fn label_action(&mut self, action: Action, cx: &mut Context<'_, Self>) {
        if !action.is_current(self.areas.labels.opening, &self.areas.labels.manager) {
            return;
        }
        match self.areas.labels.prepare(action) {
            Ok((host, ticket, ask)) => {
                self.command(vessel_app_link::Command::Labels { host, ticket, ask });
            }
            Err(failure) => self.areas.labels.error = Some(failure),
        }
        self.settle_labels(cx);
        cx.notify();
    }

    pub(crate) fn labels_answer(&mut self, event: Event, cx: &mut Context<'_, Self>) {
        let state = &mut self.areas.labels;
        if state.manager.selected() != Some(&event.host) {
            return;
        }
        match event.answer {
            // Catalog observation owns its own request/cursor lifetime; this
            // editor must never grant authority from an uncorrelated list.
            Answer::Listed(_) => {}
            Answer::Inspected(result) => {
                let _accepted = state.manager.inspected(event.ticket, result);
            }
            Answer::Updated(result) => {
                if !matches!(state.manager.state(), ManagerState::Saving { .. }) {
                    return;
                }
                // The owned observation orders catalog changes. A unary reply
                // has no cursor and must not race a list into that replica.
                let _accepted = state.manager.completed(event.ticket, result.map(|_| ()));
            }
            Answer::Deleted(result) => {
                if !matches!(state.manager.state(), ManagerState::Deleting { .. }) {
                    return;
                }
                let _accepted = state.manager.completed(event.ticket, result.map(|_| ()));
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
        let action = Action::Save(Target::new(opening, host, "Ready".parse().unwrap()));
        assert!(action.is_current(Some(opening), &manager));
        assert!(!action.is_current(Some(Opening::new(tickets.issue().unwrap())), &manager));
        manager.edit(label("Other")).unwrap();
        assert!(!action.is_current(Some(opening), &manager));
        manager.select(HostKey::random()).unwrap();
        manager.edit(label("Ready")).unwrap();
        assert!(!action.is_current(Some(opening), &manager));
    }

    #[test]
    fn labels_old_confirmation_action_cannot_consume_a_new_inspection() {
        let mut tickets = Tickets::default();
        let opening = Opening::new(tickets.issue().unwrap());
        let host = HostKey::random();
        let mut manager = Manager::default();
        manager.select(host).unwrap();
        manager.edit(label("Ready")).unwrap();
        let target = Target::new(opening, host, "Ready".parse().unwrap());
        let inspect_action = Action::Inspect(target.clone());
        let catalog = Catalog::snapshot([label("Ready")]);
        let first = tickets.issue().unwrap();
        let _target = manager.inspect(HostState::Online, &catalog, first).unwrap();
        assert!(!inspect_action.is_current(Some(opening), &manager));
        assert!(manager.inspected(first, Ok(Count::new(1))));
        let old = Action::Delete {
            target: target.clone(),
            inspection: first,
        };
        assert!(old.is_current(Some(opening), &manager));
        assert!(manager.cancel_delete());
        assert!(!old.is_current(Some(opening), &manager));
        let next = tickets.issue().unwrap();
        let _target = manager.inspect(HostState::Online, &catalog, next).unwrap();
        assert!(manager.inspected(next, Ok(Count::new(2))));
        assert!(!old.is_current(Some(opening), &manager));
        assert!(
            Action::Delete {
                target,
                inspection: next
            }
            .is_current(Some(opening), &manager)
        );
    }
}
