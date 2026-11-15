//! Correlated editor reads and writes. The same immutable request is the reply
//! authority; closing/switching a form retires its local mailbox, not actor work.
use super::definition::Save;
use crate::connection::ConnectionContext;
use crate::schedules::load::{Failure, Ticket};
use vessel_proto::agents::AgentInfo;
use vessel_proto::automation::{PreviewSchedule, ScheduleInfo, SchedulePreview, TimeZoneName};
use vessel_proto::workspace::ProjectInfo;
use vessel_proto::{GetHarnessOptions, Harness, HarnessOptions};

/// Existing typed node calls needed by a form; none starts a scheduled run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Fresh project and agent facts on the explicitly chosen machine.
    Targets,
    /// Node-cached handshake options for one chosen agent/profile.
    Options(GetHarnessOptions),
    /// Typed rule, explicit after instant and bounded count.
    Preview(PreviewSchedule),
    /// New definition or original-revision replacement, never replayed.
    Save(Box<Save>),
}
/// One form visit/edit's immutable original physical destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    connection: ConnectionContext,
    ticket: Ticket,
    operation: Operation,
}
impl Request {
    /// Inputs are already validated protocol values, not raw text or aliases.
    #[must_use]
    pub const fn on(connection: ConnectionContext, ticket: Ticket, operation: Operation) -> Self {
        Self {
            connection,
            ticket,
            operation,
        }
    }
    /// Original physical admission, fenced by the same Link actor.
    #[must_use]
    pub const fn connection(&self) -> &ConnectionContext {
        &self.connection
    }
    /// Original nonreused editor ticket.
    #[must_use]
    pub const fn ticket(&self) -> Ticket {
        self.ticket
    }
    /// Only this operation's result can settle the mailbox.
    #[must_use]
    pub const fn operation(&self) -> &Operation {
        &self.operation
    }
}
/// Fresh target data. A missing machine zone is unknown, not viewer/UTC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Targets {
    /// Projects read directly from the selected machine, not a visited page.
    pub projects: Vec<ProjectInfo>,
    /// Every configured agent, including unavailable ones.
    pub agents: Vec<AgentInfo>,
    /// Node-authored installation/structured-availability evidence.
    pub harnesses: Vec<Harness>,
    /// The selected machine's own zone when it reports one.
    pub timezone: Option<TimeZoneName>,
}
/// Actual node result; even a wrong unary payload is refused at this edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Initial project/agent/zone choices.
    Targets(Box<Targets>),
    /// Dynamic model/thinking/mode options, never hard-coded model names.
    Options(HarnessOptions),
    /// Node-only projection including a canonical interval anchor.
    Preview(SchedulePreview),
    /// Saved definition with authoritative `next_due`/state/revision.
    Saved(Box<ScheduleInfo>),
}
impl Value {
    fn matches(&self, operation: &Operation) -> bool {
        match operation {
            Operation::Targets => matches!(self, Self::Targets(_)),
            Operation::Options(_) => matches!(self, Self::Options(_)),
            Operation::Preview(input) => match self {
                Self::Preview(value) => super::preview::valid_input(input, value),
                Self::Targets(_) | Self::Options(_) | Self::Saved(_) => false,
            },
            Operation::Save(save) => match self {
                Self::Saved(info) => match save.as_ref() {
                    Save::Create(_) => true,
                    Save::Update(update) => info.id == update.schedule,
                },
                Self::Targets(_) | Self::Options(_) | Self::Preview(_) => false,
            },
        }
    }
}
#[derive(Debug, Default)]
enum Outcome {
    #[default]
    Idle,
    Waiting(Request),
    Ready {
        request: Request,
        value: Value,
    },
    Failed {
        request: Request,
        failure: Failure,
    },
}
/// One editor operation slot, reset on cadence/host/visit changes by its owner.
#[derive(Debug, Default)]
pub struct Mailbox(Outcome);
impl Mailbox {
    /// Keep one exact expected request; a newer edit makes the old reply stale.
    pub fn begin(&mut self, request: Request) {
        self.0 = Outcome::Waiting(request);
    }
    /// Navigation/connection retirement removes old target facts and readiness.
    pub fn invalidate(&mut self) {
        self.0 = Outcome::Idle;
    }
    /// Install only the expected context/ticket/operation and matching payload.
    #[must_use]
    pub fn settle(&mut self, request: &Request, answer: Result<Value, Failure>) -> bool {
        if !matches!(&self.0, Outcome::Waiting(expected) if expected == request) {
            return false;
        }
        let answer = answer.and_then(|value| {
            if value.matches(request.operation()) {
                Ok(value)
            } else {
                Err(match request.operation() {
                    Operation::Save(_) => Failure::Unconfirmed,
                    Operation::Targets | Operation::Options(_) | Operation::Preview(_) => {
                        Failure::Unexpected
                    }
                })
            }
        });
        self.0 = match answer {
            Ok(value) => Outcome::Ready {
                request: request.clone(),
                value,
            },
            Err(failure) => Outcome::Failed {
                request: request.clone(),
                failure,
            },
        };
        true
    }
    /// Reconnect retires the old form slot before the replacement Fleet arrives.
    #[must_use]
    pub fn connected(&mut self, machine: &crate::schedules::load::MachineKey) -> bool {
        if self
            .request()
            .is_some_and(|request| request.connection().machine().as_str() == machine.as_str())
        {
            self.retired()
        } else {
            false
        }
    }
    /// Retain no form payload under another/absent canonical physical admission.
    #[must_use]
    pub fn reconcile(&mut self, fleet: &crate::Fleet) -> bool {
        let retired = self.request().is_some_and(|request| {
            !fleet.machines.iter().any(|machine| {
                machine.name == request.connection().machine().as_str()
                    && machine.connection_context().as_ref() == Some(request.connection())
            })
        });
        retired && self.retired()
    }
    fn retired(&mut self) -> bool {
        let Some(request) = self.request().cloned() else {
            return false;
        };
        let failure = match &self.0 {
            Outcome::Waiting(request) if matches!(request.operation(), Operation::Save(_)) => {
                Failure::Unconfirmed
            }
            Outcome::Failed {
                failure: Failure::Unconfirmed,
                ..
            } => Failure::Unconfirmed,
            Outcome::Idle
            | Outcome::Waiting(_)
            | Outcome::Ready { .. }
            | Outcome::Failed { .. } => Failure::ConnectionChanged,
        };
        if matches!(&self.0, Outcome::Failed { failure: old, .. } if old == &failure) {
            return false;
        }
        self.0 = Outcome::Failed { request, failure };
        true
    }
    /// An unresolved operation; presence of an old request alone is not pending.
    #[must_use]
    pub const fn pending(&self) -> bool {
        matches!(self.0, Outcome::Waiting(_))
    }
    /// The exact operation that owns the visible result, failure or spinner.
    #[must_use]
    pub const fn request(&self) -> Option<&Request> {
        match &self.0 {
            Outcome::Idle => None,
            Outcome::Waiting(request)
            | Outcome::Ready { request, .. }
            | Outcome::Failed { request, .. } => Some(request),
        }
    }
    /// No older/failed/partial payload is exposed as successful form data.
    #[must_use]
    pub const fn value(&self) -> Option<&Value> {
        match &self.0 {
            Outcome::Ready { value, .. } => Some(value),
            Outcome::Idle | Outcome::Waiting(_) | Outcome::Failed { .. } => None,
        }
    }
    /// Structured presentation reason, never a wire diagnostic sentence.
    #[must_use]
    pub const fn failure(&self) -> Option<&Failure> {
        match &self.0 {
            Outcome::Failed { failure, .. } => Some(failure),
            Outcome::Idle | Outcome::Waiting(_) | Outcome::Ready { .. } => None,
        }
    }
}
#[cfg(test)]
mod tests;
