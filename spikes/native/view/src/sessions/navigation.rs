//! Addresses and request identity keep cross-host session navigation unambiguous.
use crate::machine_book::Name as NodeName;
use vessel_proto::{NodeId, SessionId};
/// A session address is stable across saved-host renames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionAddress {
    /// Authenticated destination identity.
    pub node: NodeId,
    /// Durable session on that node.
    pub session: SessionId,
}
/// Where to resolve a session; a saved name is checked against the live connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationTarget {
    /// An OS deep link or notification carries an authenticated node identity.
    Address(SessionAddress),
    /// A visible session menu knows its validated saved-host name.
    Host {
        /// Saved host, resolved to its authenticated node by link.
        host: NodeName,
        /// Session to inspect without starting a provider.
        session: SessionId,
    },
}
/// Monotonic UI request identity; old results cannot reopen a discarded target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationTicket(u64);
impl NavigationTicket {
    /// The first request in a new window.
    pub const FIRST: Self = Self(0);
    /// Exhaustion is explicit rather than reusing the identity of an old request.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Why the metadata is needed; a stale copy response must not navigate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationPurpose {
    /// Show the session through the existing follow path.
    Open,
    /// Copy a credential-free canonical session link.
    CopyLink,
}
/// One active request per window; navigation supersedes its older response.
#[derive(Debug)]
pub struct Navigation {
    issued: NavigationTicket,
    pending: Option<(NavigationTicket, NavigationPurpose)>,
}
impl Default for Navigation {
    fn default() -> Self {
        Self {
            issued: NavigationTicket::FIRST,
            pending: None,
        }
    }
}
impl Navigation {
    /// Refuse exhaustion rather than aliasing a previous request identity.
    pub fn begin(&mut self, purpose: NavigationPurpose) -> Option<NavigationTicket> {
        let ticket = self.issued.next()?;
        self.issued = ticket;
        self.pending = Some((ticket, purpose));
        Some(ticket)
    }
    /// Only the current answer may settle its control or navigate.
    pub fn finish(&mut self, ticket: NavigationTicket) -> Option<NavigationPurpose> {
        let (pending, purpose) = self.pending?;
        if pending != ticket {
            return None;
        }
        self.pending = None;
        Some(purpose)
    }
    /// Explicit navigation elsewhere revokes a previous deep-link request.
    pub fn cancel(&mut self) {
        self.pending = None;
    }
}
