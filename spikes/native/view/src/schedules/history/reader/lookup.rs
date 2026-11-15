//! A historical session read is only a candidate for the shared live opener.
//! It never allocates a resident, follows a session or resolves an alias again.
use super::{ConnectionContext, Failure, Reader, RunNumber, SessionLink, Ticket, Visit};
use vessel_proto::{GetSession, SessionInfo};

/// One original run/session lookup, bound to the same physical admission/visit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lookup {
    visit: Visit,
    run: RunNumber,
    ticket: Ticket,
    link: SessionLink,
}
impl Lookup {
    /// The actor must use its selected pre/post fence for this exact context.
    #[must_use]
    pub const fn connection(&self) -> &ConnectionContext {
        self.link.connection()
    }
    /// Existing typed session lookup; no Fleet/Card fallback.
    #[must_use]
    pub const fn input(&self) -> GetSession {
        GetSession {
            session: self.link.session(),
        }
    }
    /// Current overlay visit, distinct from a retry's request ticket.
    #[must_use]
    pub const fn visit(&self) -> Visit {
        self.visit
    }
    /// Original run that names this session.
    #[must_use]
    pub const fn run(&self) -> RunNumber {
        self.run
    }
    /// Nonreused correlation for the actual read.
    #[must_use]
    pub const fn ticket(&self) -> Ticket {
        self.ticket
    }
}
/// Exact session metadata, not permission to create a panel or start a follow.
#[derive(Debug)]
pub struct Candidate {
    request: Lookup,
    info: Box<SessionInfo>,
}
impl Candidate {
    /// Shared owner must additionally admit this original context after queue
    /// delivery and BEFORE creating its resident/layout/follow binding.
    #[must_use]
    pub const fn request(&self) -> &Lookup {
        &self.request
    }
    /// Authoritative original info, including archived/non-Fleet sessions.
    #[must_use]
    pub fn info(&self) -> &SessionInfo {
        &self.info
    }
}
#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Waiting(Lookup),
    Ready(Candidate),
    Failed(Failure),
}
#[derive(Debug, Default)]
pub(super) struct Opening(State);
impl Opening {
    pub(super) fn retire(&mut self) {
        self.0 = State::Idle;
    }
}
impl Reader {
    /// Stage only a run's actual session under the current visit. A pending
    /// lookup cannot multiply requests or allocate any live session UI.
    pub fn lookup(&mut self, expected: Visit, run: RunNumber) -> Option<Lookup> {
        let selection = self.selection.as_ref()?;
        if matches!(selection.opening.0, State::Waiting(_)) {
            return None;
        }
        let link = self.session_link(expected, run)?;
        let request = Lookup {
            visit: expected,
            run,
            ticket: self.ticket()?,
            link,
        };
        self.selection.as_mut()?.opening.0 = State::Waiting(request.clone());
        Some(request)
    }
    /// Admit metadata only for the original pending run/session/admission and id.
    /// No receipt is passed to the alias-only legacy opener.
    #[must_use]
    pub fn lookup_settled(
        &mut self,
        request: &Lookup,
        result: Result<SessionInfo, Failure>,
    ) -> bool {
        if self.session_link(request.visit, request.run).as_ref() != Some(&request.link) {
            return false;
        }
        let Some(selection) = &mut self.selection else {
            return false;
        };
        if !matches!(&selection.opening.0, State::Waiting(expected) if expected == request) {
            return false;
        }
        selection.opening.0 = match result {
            Ok(info) if info.id == request.link.session => State::Ready(Candidate {
                request: request.clone(),
                info: Box::new(info),
            }),
            Ok(_) => State::Failed(Failure::Unexpected),
            Err(failure) => State::Failed(failure),
        };
        true
    }
    /// Current metadata can be inspected, but still needs the shared owner's
    /// live admission/delivery capability before any opening side effect.
    #[must_use]
    pub fn candidate(&self, expected: Visit) -> Option<&Candidate> {
        let selection = self.selection.as_ref()?;
        if selection.visit != expected {
            return None;
        }
        match &selection.opening.0 {
            State::Ready(candidate) => Some(candidate),
            State::Idle | State::Waiting(_) | State::Failed(_) => None,
        }
    }
    /// A safe read failure, never an uncertain session write or automatic retry.
    #[must_use]
    pub fn lookup_failure(&self) -> Option<&Failure> {
        match &self.selection.as_ref()?.opening.0 {
            State::Failed(failure) => Some(failure),
            State::Idle | State::Waiting(_) | State::Ready(_) => None,
        }
    }
}
#[cfg(test)]
mod tests;
