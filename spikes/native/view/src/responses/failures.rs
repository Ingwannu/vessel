//! Read/rename failures must never replace a card's retryable response failure.
use super::{Problem, Request};
use vessel_proto::{PermissionId, QuestionId};

/// At most one visible failure per card kind, each scoped to its exact request.
/// A successor starts without its predecessor's failure; drafts stay caller-owned.
#[derive(Debug, Default)]
pub struct Failures {
    permission: Option<(PermissionId, Problem)>,
    question: Option<(QuestionId, Problem)>,
}
impl Failures {
    /// Record a safe category, never an opaque transport/provider diagnostic.
    pub fn failed(&mut self, request: Request, problem: Problem) {
        match request {
            Request::Permission(id) => self.permission = Some((id, problem)),
            Request::Question(id) => self.question = Some((id, problem)),
        }
    }
    /// Only the named request may display this failure.
    #[must_use]
    pub fn get(&self, request: Request) -> Option<Problem> {
        match request {
            Request::Permission(id) => self
                .permission
                .filter(|(asked, _)| *asked == id)
                .map(|(_, problem)| problem),
            Request::Question(id) => self
                .question
                .filter(|(asked, _)| *asked == id)
                .map(|(_, problem)| problem),
        }
    }
    /// A retry/ack clears its own failure without touching the other card.
    pub fn clear(&mut self, request: Request) {
        if self.get(request).is_none() {
            return;
        }
        match request {
            Request::Permission(_) => self.permission = None,
            Request::Question(_) => self.question = None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        failures.clear(question);
        assert_eq!(failures.get(successor), Some(Problem::Unsent));
        failures.clear(permission);
        assert_eq!(failures.get(permission), None);
        assert_eq!(failures.get(successor), Some(Problem::Unsent));
    }
}
