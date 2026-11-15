//! Opening identity admits a real close before a shared control acknowledges focus.
use super::RequestFailure;
use crate::devices::{Ticket, Tickets};

/// One usage overlay cycle; the area's existing allocator owns its identity.
#[derive(Debug, Default)]
pub struct ModalCycle(Option<Ticket>);
impl ModalCycle {
    /// Repeated open actions focus the current cycle rather than replacing its opener.
    /// # Errors
    /// Exhausted identities refuse opening instead of admitting an old close.
    pub fn show(&mut self, tickets: &mut Tickets) -> Result<Ticket, RequestFailure> {
        if let Some(current) = self.0 {
            return Ok(current);
        }
        let ticket = tickets
            .next_ticket()
            .map_err(|_exhausted| RequestFailure::Exhausted)?;
        self.0 = Some(ticket);
        Ok(ticket)
    }
    /// Capture this in every rendered dismissal path.
    #[must_use]
    pub const fn current(&self) -> Option<Ticket> {
        self.0
    }
    /// Only an admitted actual close may acknowledge shared focus or snap state.
    #[must_use]
    pub fn dismiss(&mut self, expected: Ticket) -> bool {
        if self.0 != Some(expected) {
            return false;
        }
        self.0 = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_close_cannot_dismiss_a_reopened_overlay_and_repeated_show_is_idempotent() {
        let mut cycle = ModalCycle::default();
        let mut tickets = Tickets::default();
        let first = cycle.show(&mut tickets).unwrap();
        assert_eq!(cycle.show(&mut tickets).unwrap(), first);
        assert!(cycle.dismiss(first));
        assert_eq!(cycle.current(), None);
        let second = cycle.show(&mut tickets).unwrap();
        assert_ne!(first, second);
        assert!(!cycle.dismiss(first));
        assert_eq!(cycle.current(), Some(second));
        assert!(cycle.dismiss(second));
    }
}
