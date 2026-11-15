//! Manager intents use the shell's existing bounded queue/refusal settlement.
//! No draft or catalog changes optimistically; results retain host and ticket.

use vessel_app_kit::ui::Context;
use vessel_app_link::labels::{Answer, Command, Event};
use vessel_app_view::labels::{Failure, HostKey, ManagerState, Ticket};

use super::Labels;
use crate::shell::Shell;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Save,
    Inspect,
    Delete,
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
            Action::Save => {
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
            Action::Inspect => {
                Command::InspectDelete(self.manager.inspect(host.state, &host.catalog, ticket)?)
            }
            Action::Delete => Command::Delete(self.manager.delete(host.state, ticket)?),
        };
        self.error = None;
        Ok((host.key, ticket, ask))
    }
}

impl Shell {
    pub(super) fn label_action(&mut self, action: Action, cx: &mut Context<'_, Self>) {
        match self.areas.labels.prepare(action) {
            Ok((host, ticket, ask)) => {
                self.command(vessel_app_link::Command::Labels { host, ticket, ask })
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
                let original = state
                    .manager
                    .state()
                    .draft()
                    .map(|draft| draft.original().name.clone());
                let settled = result.as_ref().map(|_| ()).map_err(|failure| *failure);
                if state.manager.completed(event.ticket, settled)
                    && let Ok(label) = result
                    && let Some(host) = state.hosts.iter_mut().find(|host| host.key == event.host)
                {
                    host.catalog.upsert(original.as_ref(), label);
                }
            }
            Answer::Deleted(result) => {
                if !matches!(state.manager.state(), ManagerState::Deleting { .. }) {
                    return;
                }
                let original = state
                    .manager
                    .state()
                    .draft()
                    .map(|draft| draft.original().name.clone());
                if state.manager.completed(event.ticket, result.map(|_| ()))
                    && result.is_ok()
                    && let Some(name) = original
                    && let Some(host) = state.hosts.iter_mut().find(|host| host.key == event.host)
                {
                    host.catalog.remove(&name);
                }
            }
        }
        self.settle_labels(cx);
    }
}
