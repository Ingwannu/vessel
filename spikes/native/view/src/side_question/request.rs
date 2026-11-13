//! Request identities last for the app's lifetime. Closing, resetting or
//! renaming a panel must not let an older answer restore its previous side.

use vessel_proto::ErrorCode;

/// One request or stream generation, never reused by its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ticket(u64);

/// The app retains this source when a panel is closed or its target moves.
#[derive(Debug, Default)]
pub struct Tickets(u64);

/// All identities were used; reopening the app creates a new channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("side question request identities exhausted")]
pub struct Exhausted;

impl Tickets {
    /// Reserve a fresh identity, refusing rather than accepting an old reply.
    ///
    /// # Errors
    /// The app has used every available identity.
    pub fn next_ticket(&mut self) -> Result<Ticket, Exhausted> {
        self.0 = self.0.checked_add(1).ok_or(Exhausted)?;
        Ok(Ticket(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities_are_never_reused_even_after_exhaustion() {
        let mut tickets = Tickets::default();
        assert_ne!(
            tickets.next_ticket().unwrap(),
            tickets.next_ticket().unwrap()
        );
        let mut last = Tickets(u64::MAX - 1);
        assert_eq!(last.next_ticket(), Ok(Ticket(u64::MAX)));
        assert_eq!(last.next_ticket(), Err(Exhausted));
        assert_eq!(last.next_ticket(), Err(Exhausted));
    }
}

/// Safe reasons, localized in view; no peer-supplied message is retained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Failure {
    /// The machine has no current connection.
    #[error("the machine is not connected")]
    Offline,
    /// Its typed refusal, excluding the remote error text.
    #[error("the node refused the side operation ({0:?})")]
    Refused(ErrorCode),
    /// A response does not fit the requested operation.
    #[error("the machine returned an invalid side response")]
    InvalidReply,
    /// The call deadline expired; a write may already have happened.
    #[error("the side operation did not answer in time")]
    TimedOut,
    /// This app or connection cannot safely reserve another request id.
    #[error("side question request identities exhausted")]
    Exhausted,
    /// The link has no room for another task or request.
    #[error("the side operation queue is full")]
    Busy,
}

impl From<Exhausted> for Failure {
    fn from(_exhausted: Exhausted) -> Self {
        Self::Exhausted
    }
}
