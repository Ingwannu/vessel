//! Local creation identities; these are never serialized as wire request ids.

/// Identifies one draft submission until every consumer has settled it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ticket(u64);

impl Ticket {
    /// Start a local sequence without reserving any value as a sentinel.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The opaque identity for keyed UI elements and local diagnostics.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    /// Advance without ever reusing a ticket after counter exhaustion.
    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Ticket;

    #[test]
    fn identities_never_wrap_or_reserve_zero() {
        assert_eq!(Ticket::new(0).checked_next(), Some(Ticket::new(1)));
        assert_eq!(Ticket::new(u64::MAX).checked_next(), None);
    }
}
