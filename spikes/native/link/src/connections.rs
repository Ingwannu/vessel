//! The existing connection registry owns physical admission, never the screen.
//!
//! [Decision Log]
//! - Purpose: retire callbacks before an observed book replacement can dial.
//! - Constraint: a session-read refusal is not failed authentication.
//! - Alternatives: reuse aliases blindly or make a Devices connection cache.
//! - Choice: keep parsed binding and validity on the one connected record.
//! - Why: immutable clients cannot be redirected by a later alias lookup.
//! - Cost: a reload invalidates old intent even if it reconnects to the same node.
use crate::{Event, State};
use tokio::sync::watch;
use vessel_app_view::Agent;
use vessel_app_view::connection::ConnectionContext;
use vessel_client::{NodeClient, book::BookEntry};

/// A physical client and its actor-owned, publicly exportable admission.
#[derive(Clone)]
pub(crate) struct Connected {
    pub(crate) client: NodeClient,
    pub(crate) agents: Vec<Agent>,
    pub(crate) inbox_presence: crate::inbox::PresenceHub,
    binding: BookEntry,
    pub(crate) context: ConnectionContext,
    validity: watch::Sender<Validity>,
    lease: std::sync::Arc<SelectedLease>,
}

/// Retirement is irreversible for an admitted generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Validity {
    Active,
    Retired,
}

impl Connected {
    pub(crate) fn new(
        client: NodeClient,
        agents: Vec<Agent>,
        binding: BookEntry,
        context: ConnectionContext,
    ) -> Self {
        let (validity, receiver) = watch::channel(Validity::Active);
        let lease = std::sync::Arc::new(SelectedLease::new(
            client.clone(),
            context.clone(),
            receiver,
        ));
        Self {
            client,
            agents,
            inbox_presence: crate::inbox::PresenceHub::default(),
            binding,
            context,
            validity,
            lease,
        }
    }
    pub(crate) fn retire(&self) {
        let _old = self.validity.send_replace(Validity::Retired);
    }
    #[cfg(test)]
    pub(crate) fn validity(&self) -> watch::Receiver<Validity> {
        self.validity.subscribe()
    }
    pub(crate) fn matches(&self, expected: &ConnectionContext) -> bool {
        // Identity admission is distinct from a closed transport. The selected
        // call reports Closed once through the actor's retirement command.
        self.lease.matches(expected)
    }
    pub(crate) fn renamed(&self, context: ConnectionContext) -> Self {
        self.retire();
        Self::new(
            self.client.clone(),
            self.agents.clone(),
            self.binding.clone(),
            context,
        )
    }
    fn retains(&mut self, entry: &BookEntry) -> bool {
        if !self.matches(&self.context) {
            return false;
        }
        // Like Paseo's learned security floor: learning the already enforced
        // authenticated id keeps the client; any route/credential change does not.
        if self.binding == *entry {
            return true;
        }
        let mut learned = self.binding.clone();
        if learned.id.is_none() && entry.id == Some(self.client.node().id) {
            learned.id = entry.id;
            if learned == *entry {
                self.binding = entry.clone();
                return true;
            }
        }
        false
    }
}

mod edits;
mod selected;
mod snapshots;
pub(crate) use selected::{RequestFailure, SelectedConnection, SelectedLease};

impl State {
    pub(crate) async fn closed_connection(
        &mut self,
        tag: &vessel_app_view::connection::ConnectionTag,
    ) {
        let current = self
            .clients
            .get(tag.machine().as_str())
            .is_some_and(|connected| connected.context.epoch() == tag.epoch());
        if current && let Some(connected) = self.clients.remove(tag.machine().as_str()) {
            connected.retire();
            self.emit(Event::DeviceRetired(tag.clone())).await;
        }
    }
    pub(crate) fn admit_connection(
        &mut self,
        name: &str,
        entry: &BookEntry,
        client: NodeClient,
        agents: Vec<Agent>,
    ) -> Result<Connected, crate::LinkError> {
        let context = self
            .admissions
            .admit(
                vessel_app_view::connection::MachineKey::from_book(name),
                client.node().id,
            )
            .map_err(|_exhausted| crate::LinkError::NotConnected(name.to_owned()))?;
        Ok(Connected::new(client, agents, entry.clone(), context))
    }
    pub(crate) async fn retire_connections(&mut self) {
        let retiring = std::mem::take(&mut self.clients);
        for connected in retiring.values() {
            connected.retire();
        }
        for connected in retiring.values() {
            self.emit(Event::DeviceRetired(connected.context.tag()))
                .await;
        }
    }
    pub(crate) async fn reconcile_connections(
        &mut self,
        book: &[(vessel_client::book::NodeName, BookEntry)],
    ) {
        let mut retired = Vec::new();
        self.clients.retain(|name, connected| {
            let keep = book
                .iter()
                .find(|(key, _)| key.as_str() == name)
                .is_some_and(|(_, entry)| connected.retains(entry));
            if !keep {
                connected.retire();
                retired.push(connected.context.tag());
            }
            keep
        });
        for tag in retired {
            self.emit(Event::DeviceRetired(tag)).await;
        }
    }
}

#[cfg(test)]
mod tests;
