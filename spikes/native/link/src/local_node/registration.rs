//! Ephemeral fleet registration never rewrites or shadows the user's node book.

use super::Connection;
use crate::{Event, State};
use vessel_app_view::Machine;
use vessel_app_view::devices::Ticket;
use vessel_app_view::local_node::{AdmissionProblem, LocalTarget};

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

impl State {
    pub(crate) async fn register_local(&mut self, ask: Ask) {
        // The previous branch-only adapter predates canonical Connected leases.
        // Refuse explicitly until the lead grants the actual ManagedLocal slot;
        // a fabricated BookEntry or a second epoch would create false authority.
        let result = if ask.connection.is_some() {
            Err(AdmissionProblem::Unavailable)
        } else {
            Ok(None)
        };
        self.emit(Event::LocalNode(Answer {
            ticket: ask.ticket,
            result,
        }))
        .await;
    }

    pub(crate) async fn check_local_collision<'a>(
        &mut self,
        _names: impl Iterator<Item = &'a str>,
    ) {
        // No local admission exists while this explicit unavailable skeleton is active.
    }

    pub(crate) async fn local_machine(&mut self) -> Option<Machine> {
        None
    }
}
