//! A read-only run-history visit, bound to the selected physical connection.
//!
//! [Decision Log]
//! - Purpose: a schedule's history and session links must not attach to another
//!   machine under the same book key after a reconnect or a late reply.
//! - Existing assets: ConnectionContext, nonreusing Tickets, RunList's wire cap,
//!   and the existing outcome/session classifiers. No write or polling engine.
//! - Choice: a private selection and exact request; only that pending request
//!   admits a complete newest-first list. Reconnect retires rows and links.
//! - Why/cost: history remains available to read-only devices and ended rules;
//!   a failed read is explicit and retry requires a current visit activation.
use crate::connection::ConnectionContext;
use crate::schedules::{
    load::{Failure, MachineKey, Ticket, Tickets},
    page::{Body, Row},
};
use vessel_proto::automation::{ListRuns, RunList, RunNumber, ScheduleRun};
use vessel_proto::{ScheduleId, SessionId};
mod lookup;
use lookup::Opening;
pub use lookup::{Candidate, Lookup};

/// One history overlay visit, separate from a retry's request identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visit(Ticket);
/// Exact original history request, echoed by Link even on local refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    connection: ConnectionContext,
    ticket: Ticket,
    input: ListRuns,
}
impl Request {
    /// The actor must fence this original admission, never resolve a new alias.
    #[must_use]
    pub const fn connection(&self) -> &ConnectionContext {
        &self.connection
    }
    /// Per-read correlation, not the schedule's revision or run number.
    #[must_use]
    pub const fn ticket(&self) -> Ticket {
        self.ticket
    }
    /// Existing typed read call, including the originally selected schedule.
    #[must_use]
    pub const fn input(&self) -> ListRuns {
        self.input
    }
}
/// A session link from the authoritative outcome, under the original admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLink {
    connection: ConnectionContext,
    session: SessionId,
}
impl SessionLink {
    /// Reject this link when the canonical physical admission no longer matches.
    #[must_use]
    pub const fn connection(&self) -> &ConnectionContext {
        &self.connection
    }
    /// Session named by the run, including failed creation that made one.
    #[must_use]
    pub const fn session(&self) -> SessionId {
        self.session
    }
}
#[derive(Debug)]
enum Outcome {
    Waiting(Request),
    Ready { request: Request, list: RunList },
    Failed { request: Request, failure: Failure },
}
impl Outcome {
    fn request(&self) -> &Request {
        match self {
            Self::Waiting(request) | Self::Ready { request, .. } | Self::Failed { request, .. } => {
                request
            }
        }
    }
}
#[derive(Debug)]
struct Selection {
    visit: Visit,
    title: String,
    outcome: Outcome,
    opening: Opening,
}
/// Read-only history owner; its ticket issuer survives close and reopen.
#[derive(Debug, Default)]
pub struct Reader {
    tickets: Tickets,
    selection: Option<Selection>,
    /// Exhausted identities do not silently recycle a delayed request id.
    pub local_failure: Option<Failure>,
}
impl Reader {
    fn ticket(&mut self) -> Option<Ticket> {
        let ticket = self.tickets.issue();
        if ticket.is_none() {
            self.local_failure = Some(Failure::Exhausted);
        }
        ticket
    }
    /// Read-only and ended rows remain inspectable; an offline cached row does
    /// not authorize a request or silently choose a replacement physical node.
    pub fn open(&mut self, row: &Row<'_>) -> Option<Request> {
        let connection = row.connection?.clone();
        let visit = Visit(self.ticket()?);
        let request = Request {
            connection,
            ticket: self.ticket()?,
            input: ListRuns {
                schedule: row.schedule.id,
            },
        };
        self.selection = Some(Selection {
            visit,
            title: crate::schedules::title(&row.schedule.definition).to_owned(),
            outcome: Outcome::Waiting(request.clone()),
            opening: Opening::default(),
        });
        Some(request)
    }
    /// Only the visible visit can refresh/retry. A pending read cannot multiply
    /// work; unsupported/exhausted failures need an external fix rather than retry.
    pub fn retry(&mut self, expected: Visit) -> Option<Request> {
        let selection = self.selection.as_ref()?;
        if !self.can_retry(expected) {
            return None;
        }
        let mut request = selection.outcome.request().clone();
        request.ticket = self.ticket()?;
        let selection = self.selection.as_mut()?;
        selection.opening.retire();
        selection.outcome = Outcome::Waiting(request.clone());
        Some(request)
    }
    /// Drawing and dispatch use the same pending/retired/failure retry rule.
    #[must_use]
    pub fn can_retry(&self, expected: Visit) -> bool {
        self.visit() == Some(expected)
            && self.body() != Some(Body::Loading)
            && self.local_failure.is_none()
            && self
                .failure()
                .is_none_or(|failure| failure.retryable() && *failure != Failure::ConnectionChanged)
    }
    /// No partial, duplicate, ascending or oversized payload is installed.
    #[must_use]
    pub fn settle(&mut self, request: &Request, result: Result<RunList, Failure>) -> bool {
        let Some(selection) = &mut self.selection else {
            return false;
        };
        if !matches!(&selection.outcome, Outcome::Waiting(expected) if expected == request) {
            return false;
        }
        let result = result.and_then(|list| {
            if valid(&list) {
                Ok(list)
            } else {
                Err(Failure::Unexpected)
            }
        });
        selection.outcome = match result {
            Ok(list) => Outcome::Ready {
                request: request.clone(),
                list,
            },
            Err(failure) => Outcome::Failed {
                request: request.clone(),
                failure,
            },
        };
        true
    }
    /// Retire even when the later Fleet has identical labels and node ids.
    #[must_use]
    pub fn connected(&mut self, machine: &MachineKey) -> bool {
        let affected = self.selection.as_ref().is_some_and(|selection| {
            selection.outcome.request().connection.machine().as_str() == machine.as_str()
        });
        affected && self.retired()
    }
    /// Canonical node+epoch, not the display name alone, owns the selection.
    #[must_use]
    pub fn reconcile(&mut self, fleet: &crate::Fleet) -> bool {
        let retired = self.selection.as_ref().is_some_and(|selection| {
            let original = selection.outcome.request().connection();
            !fleet
                .machines
                .iter()
                .any(|machine| machine.connection_context().as_ref() == Some(original))
        });
        retired && self.retired()
    }
    fn retired(&mut self) -> bool {
        let Some(selection) = &mut self.selection else {
            return false;
        };
        if matches!(
            &selection.outcome,
            Outcome::Failed {
                failure: Failure::ConnectionChanged,
                ..
            }
        ) {
            return false;
        }
        selection.opening.retire();
        selection.outcome = Outcome::Failed {
            request: selection.outcome.request().clone(),
            failure: Failure::ConnectionChanged,
        };
        true
    }
    /// A retained close callback cannot unmount another history visit.
    pub fn close(&mut self, expected: Visit) -> bool {
        if self.visit() != Some(expected) {
            return false;
        }
        self.selection = None;
        true
    }
    /// Current overlay identity; no identity is inferred from its title.
    #[must_use]
    pub fn visit(&self) -> Option<Visit> {
        self.selection.as_ref().map(|selection| selection.visit)
    }
    /// Title captured from the original typed schedule, without a fresh lookup.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.selection
            .as_ref()
            .map(|selection| selection.title.as_str())
    }
    /// Loading/empty/failed/content are distinct; a closed history is unmounted.
    #[must_use]
    pub fn body(&self) -> Option<Body> {
        self.selection
            .as_ref()
            .map(|selection| match &selection.outcome {
                Outcome::Waiting(_) => Body::Loading,
                Outcome::Ready { list, .. } => {
                    if list.runs.is_empty() {
                        Body::Empty
                    } else {
                        Body::Content
                    }
                }
                Outcome::Failed { .. } => Body::Failed,
            })
    }
    /// Only admitted authoritative rows remain linkable.
    #[must_use]
    pub fn runs(&self) -> &[ScheduleRun] {
        match self.selection.as_ref().map(|selection| &selection.outcome) {
            Some(Outcome::Ready { list, .. }) => &list.runs,
            Some(Outcome::Waiting(_) | Outcome::Failed { .. }) | None => &[],
        }
    }
    /// Visible typed failure, translated through the existing area mapper.
    #[must_use]
    pub fn failure(&self) -> Option<&Failure> {
        match self.selection.as_ref().map(|selection| &selection.outcome) {
            Some(Outcome::Failed { failure, .. }) => Some(failure),
            Some(Outcome::Waiting(_) | Outcome::Ready { .. }) | None => None,
        }
    }
    /// Activate only a current run that actually names a session; making,
    /// skipped, interrupted and unknown outcomes never invent a link.
    #[must_use]
    pub fn session_link(&self, expected: Visit, run: RunNumber) -> Option<SessionLink> {
        let selection = self.selection.as_ref()?;
        if selection.visit != expected {
            return None;
        }
        let session = self
            .runs()
            .iter()
            .find(|item| item.number == run)
            .and_then(|item| super::session(&item.outcome))?;
        Some(SessionLink {
            connection: selection.outcome.request().connection.clone(),
            session,
        })
    }
    /// Original schedule identity remains available in failed/empty history.
    #[must_use]
    pub fn schedule(&self) -> Option<ScheduleId> {
        self.selection
            .as_ref()
            .map(|selection| selection.outcome.request().input.schedule)
    }
}
fn valid(list: &RunList) -> bool {
    list.runs.len() <= RunList::MAX
        && list.runs.windows(2).all(|pair| match pair {
            [newer, older] => newer.number > older.number,
            _ => false,
        })
}
#[cfg(test)]
mod tests;
