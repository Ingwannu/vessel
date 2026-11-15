//! Read/rename failures must never replace a card's retryable response failure.
use super::{Problem, Request};
use std::collections::HashMap;

/// One visible failure per exact request, independent of simultaneous cards.
/// A successor starts without its predecessor's failure; drafts stay caller-owned.
#[derive(Debug, Default)]
pub struct Failures {
    requests: HashMap<Request, Problem>,
}
impl Failures {
    /// No response error needs reconciliation.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }
    /// Retain each request's refusal independently of its siblings.
    pub fn failed(&mut self, request: Request, problem: Problem) {
        let _previous = self.requests.insert(request, problem);
    }
    /// Only the named request may display this failure.
    #[must_use]
    pub fn get(&self, request: Request) -> Option<Problem> {
        self.requests.get(&request).copied()
    }
    /// A snapshot still listing the request cannot prove an uncertain effect did
    /// not happen. Only exact response/authority reconciliation clears this hold;
    /// other admission and provider-delivery checks remain independently required.
    #[must_use]
    pub fn retryable(&self, request: Request) -> bool {
        self.get(request) != Some(Problem::Unconfirmed)
    }
    /// A retry/ack clears its own failure without touching another request.
    pub fn clear(&mut self, request: Request) {
        let _previous = self.requests.remove(&request);
    }
    /// The session owner drops failures only when their request is no longer pending.
    pub fn retain(&mut self, pending: &[Request]) {
        self.requests.retain(|request, _| pending.contains(request));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{PermissionId, QuestionId};
    #[test]
    fn independent_failures_do_not_leak_to_successors_or_clear_each_other() {
        let mut failures = Failures::default();
        let permission = Request::Permission(PermissionId::random());
        let question = Request::Question(QuestionId::random());
        let successor = Request::Question(QuestionId::random());
        failures.failed(permission, Problem::Offline);
        failures.failed(question, Problem::Unconfirmed);
        assert_eq!(failures.get(permission), Some(Problem::Offline));
        assert_eq!(failures.get(question), Some(Problem::Unconfirmed));
        assert_eq!(failures.get(successor), None);
        failures.failed(successor, Problem::Unsent);
        assert_eq!(failures.get(question), Some(Problem::Unconfirmed));
        failures.clear(question);
        assert_eq!(failures.get(successor), Some(Problem::Unsent));
        failures.clear(permission);
        assert_eq!(failures.get(permission), None);
        assert_eq!(failures.get(successor), Some(Problem::Unsent));
    }
}
