//! Correlating editor I/O independently of file, pane and local model
//! sequence. Reopening the same path must not accept an old reply.

/// One editor request's process-local identity, echoed by the link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileTicket(pub u64);

/// The app cannot reuse a request identity, even after integer exhaustion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("file request identities are exhausted")]
pub struct TicketsExhausted;

/// A monotonic source kept for the entire file area's lifetime.
#[derive(Debug, Default)]
pub struct FileTickets(Option<u64>);

impl FileTickets {
    /// Allocate a fresh identity or refuse without wrapping an old one.
    ///
    /// # Errors
    /// [`TicketsExhausted`] after every identity has been used.
    pub fn issue(&mut self) -> Result<FileTicket, TicketsExhausted> {
        let next = match self.0 {
            None => 0,
            Some(previous) => previous.checked_add(1).ok_or(TicketsExhausted)?,
        };
        self.0 = Some(next);
        Ok(FileTicket(next))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities_are_unique_and_exhaustion_never_wraps() {
        let mut tickets = FileTickets::default();
        assert_ne!(tickets.issue().unwrap(), tickets.issue().unwrap());
        tickets.0 = Some(u64::MAX);
        assert!(tickets.issue().is_err());
        assert!(tickets.issue().is_err());
    }
}
