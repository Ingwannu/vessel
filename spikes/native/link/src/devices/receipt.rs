//! A completed reply cannot expose data until its original weak lease is checked.
//! Constructors stay at the actor edge: no unguarded success or fake guard can
//! be supplied by an App fixture, and a stale credential is dropped unopened.
use super::{Ask, ConnectionContext, Kind, Outcome, Problem, Ticket};
use crate::DeliveryGuard;
use vessel_app_view::connection::Refusal;

#[derive(Clone, Copy)]
pub(super) struct Pending {
    kind: Kind,
    ticket: Ticket,
}
impl Pending {
    pub(super) fn new(ask: &Ask, ticket: Ticket) -> Self {
        Self {
            kind: ask.kind(),
            ticket,
        }
    }
    fn failed(self, problem: Problem) -> Failed {
        Failed {
            pending: self,
            problem,
        }
    }
}
/// A correlated failure can yield only an error outcome, never successful data.
#[derive(Debug)]
pub struct Failed {
    pending: Pending,
    problem: Problem,
}
impl Failed {
    /// Safe closed failure, without any untrusted peer diagnostic.
    #[must_use]
    pub const fn problem(&self) -> Problem {
        self.problem
    }
    /// Preserve original request correlation while producing only failure data.
    #[must_use]
    pub fn into_outcome(self) -> Outcome {
        self.pending.kind.refused(self.pending.ticket, self.problem)
    }
}
impl std::fmt::Debug for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pending(<correlated>)")
    }
}
#[derive(Debug)]
enum Body {
    Completed {
        pending: Pending,
        guard: DeliveryGuard,
        outcome: Outcome,
    },
    Refused(Failed),
    Retired(Failed),
}
/// An actor-produced, sealed reply; no public constructor can attach a guard.
#[derive(Debug)]
pub struct Answer {
    context: ConnectionContext,
    body: Body,
}
/// The receiver checks after queue delivery, before installing any facts.
#[derive(Debug)]
pub enum Received {
    /// The original weak lease is valid at this synchronous receive boundary.
    Completed {
        /// Original admission, not a later alias resolution.
        context: ConnectionContext,
        /// Keep weak validity for subsequent render/effect callbacks.
        guard: DeliveryGuard,
        /// Safe typed result, with a masked invitation if one was created.
        outcome: Outcome,
    },
    /// Selection/capacity refused before a worker could dispatch.
    Refused {
        /// Original correlation, never a guessed replacement.
        context: ConnectionContext,
        /// This can produce no success value.
        failure: Failed,
    },
    /// Admission retired; a sent write may already have taken effect.
    Retired {
        /// Only this exact old admission may be retired in the UI.
        context: ConnectionContext,
        /// No stale data/credential is retained in this projection.
        failure: Failed,
    },
}
impl Answer {
    pub(super) fn completed(
        context: ConnectionContext,
        pending: Pending,
        guard: DeliveryGuard,
        outcome: Outcome,
    ) -> Self {
        Self {
            context,
            body: Body::Completed {
                pending,
                guard,
                outcome,
            },
        }
    }
    pub(super) fn refused(context: ConnectionContext, pending: Pending, problem: Problem) -> Self {
        Self {
            context,
            body: Body::Refused(pending.failed(problem)),
        }
    }
    pub(super) fn retired(context: ConnectionContext, pending: Pending, problem: Problem) -> Self {
        Self {
            context,
            body: Body::Retired(pending.failed(problem)),
        }
    }
    /// Immutable request facts, safe even when the reply has become stale.
    #[must_use]
    pub const fn context(&self) -> &ConnectionContext {
        &self.context
    }
    /// Validate the SAME weak lease after queue delivery, with no I/O/await.
    /// Invalid results (including invitations) are dropped before exposure.
    #[must_use]
    pub fn receive(self) -> Received {
        let Self { context, body } = self;
        match body {
            Body::Completed {
                pending,
                guard,
                outcome,
            } => match guard.validate(&context) {
                Ok(()) => Received::Completed {
                    context,
                    guard,
                    outcome,
                },
                Err(why) => {
                    drop(outcome);
                    Received::Retired {
                        context,
                        failure: pending.failed(problem(why)),
                    }
                }
            },
            Body::Refused(failure) => Received::Refused { context, failure },
            Body::Retired(failure) => Received::Retired { context, failure },
        }
    }
}
pub(super) const fn problem(why: Refusal) -> Problem {
    match why {
        Refusal::Changed => Problem::TargetChanged,
        Refusal::Closed => Problem::Offline,
    }
}
