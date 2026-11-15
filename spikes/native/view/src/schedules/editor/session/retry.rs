//! Explicit read retries keep the original visit and admission; writes are never replayed.
use super::{Failure, Model, State, Visit, io};
impl Model {
    /// Repeat only the failed current read captured by a visible retry action.
    /// A replaced form, newer request or uncertain write cannot use this path.
    pub fn retry_read(&mut self, visit: Visit, expected: &io::Request) -> Option<io::Request> {
        if self.visit() != Some(visit) || self.blocked() {
            return None;
        }
        let slot = self.mailbox(expected.operation())?;
        if slot.request() != Some(expected)
            || !slot.failure().is_some_and(|failure| {
                failure.retryable() && !matches!(failure, Failure::ConnectionChanged)
            })
        {
            return None;
        }
        match expected.operation() {
            io::Operation::Save(_) => return None,
            io::Operation::Targets | io::Operation::Options(_) | io::Operation::Preview(_) => {}
        }
        let request = io::Request::on(
            expected.connection().clone(),
            self.ticket()?,
            expected.operation().clone(),
        );
        if let State::Active(active) = &mut self.state {
            let slot = match expected.operation() {
                io::Operation::Targets => &mut active.targets,
                io::Operation::Options(_) => &mut active.options,
                io::Operation::Preview(_) => &mut active.preview,
                io::Operation::Save(_) => return None,
            };
            slot.begin(request.clone());
        }
        Some(request)
    }
}
