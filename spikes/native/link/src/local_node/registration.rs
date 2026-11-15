//! Ephemeral fleet registration never rewrites or shadows the user's node book.

use super::Connection;
use crate::{Connected, Event, State};
use vessel_app_view::devices::Ticket;
use vessel_app_view::local_node::{AdmissionProblem, LocalTarget};
use vessel_app_view::machine_book::Name;
use vessel_app_view::{Card, Machine, NodeVersion, Reach};

/// Every registration response carries the window's own current ticket.
#[derive(Debug)]
pub struct Ask {
    /// Correlates a delayed acknowledgement with the current watch snapshot.
    pub ticket: Ticket,
    /// None removes only the previous ephemeral registration.
    pub connection: Option<Connection>,
}

/// A target exists only after collision checks and authenticated registration.
#[derive(Debug)]
pub struct Answer {
    /// Original ticket; stale replies cannot republish a stopped connection.
    pub ticket: Ticket,
    /// Ephemeral destination or typed refusal; never remote book contents.
    pub result: Result<Option<LocalTarget>, AdmissionProblem>,
}

impl Ask {
    /// Settle a full/closed UI queue without leaving a pending local target.
    #[must_use]
    pub fn refused(self) -> Answer {
        Answer {
            ticket: self.ticket,
            result: Err(AdmissionProblem::Unsent),
        }
    }
}

pub(crate) struct Registration {
    target: LocalTarget,
    connected: Connected,
    ticket: Ticket,
}

impl State {
    pub(crate) async fn register_local(&mut self, ask: Ask) {
        let result = self.install_local(ask.ticket, ask.connection).await;
        self.emit(Event::LocalNode(Answer {
            ticket: ask.ticket,
            result,
        }))
        .await;
        self.refresh().await;
    }

    async fn install_local(
        &mut self,
        ticket: Ticket,
        connection: Option<Connection>,
    ) -> Result<Option<LocalTarget>, AdmissionProblem> {
        if let Some(previous) = self.local_registration.take() {
            let _old = self.clients.remove(previous.target.machine.as_str());
        }
        let Some(connection) = connection else {
            return Ok(None);
        };
        let name = Name::parse(&format!("desktop-{}", connection.client.node().id))
            .map_err(|_invalid| AdmissionProblem::Unavailable)?;
        let book = self
            .saved_entries()
            .await
            .map_err(|_read| AdmissionProblem::BookRead)?;
        if book
            .iter()
            .any(|(saved, _entry)| saved.as_str() == name.as_str())
        {
            return Err(AdmissionProblem::BookCollision);
        }
        let target = LocalTarget {
            node: connection.client.node().id,
            machine: name,
            provenance: connection.provenance,
        };
        let connected = self.describe_client(connection.client).await;
        let _old = self
            .clients
            .insert(target.machine.as_str().to_owned(), connected.clone());
        self.local_registration = Some(Registration {
            target: target.clone(),
            connected,
            ticket,
        });
        Ok(Some(target))
    }

    /// A saved name created after registration wins. Never dial it through the
    /// local connection just because an earlier ephemeral entry used that name.
    pub(crate) async fn check_local_collision<'a>(&mut self, names: impl Iterator<Item = &'a str>) {
        let collision = self.local_registration.as_ref().is_some_and(|local| {
            names
                .into_iter()
                .any(|name| name == local.target.machine.as_str())
        });
        if collision && let Some(local) = self.local_registration.take() {
            let _old = self.clients.remove(local.target.machine.as_str());
            self.emit(Event::LocalNode(Answer {
                ticket: local.ticket,
                result: Err(AdmissionProblem::BookCollision),
            }))
            .await;
        }
    }

    pub(crate) async fn local_machine(&mut self) -> Option<Machine> {
        let local = self.local_registration.as_ref()?;
        let name = local.target.machine.as_str().to_owned();
        let connected = local.connected.clone();
        let sessions = connected.client.sessions().await;
        match sessions {
            Ok(sessions) => {
                let machine = Machine {
                    name: name.clone(),
                    reach: Reach::Online,
                    node: Some(connected.client.node().id),
                    protocol: Some(connected.client.protocol()),
                    version: Some(NodeVersion::new(&connected.client.node().version)),
                    agents: connected.agents.clone(),
                    sessions: sessions
                        .iter()
                        .map(|session| Card::from_session(&name, session))
                        .collect(),
                };
                let _old = self.clients.insert(name, connected);
                Some(machine)
            }
            Err(error) => {
                let _old = self.clients.remove(&name);
                Some(Machine::unreachable(name, &error.to_string()))
            }
        }
    }
}
