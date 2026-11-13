//! A Devices page is scoped to one already-parsed node-book entry.
//! Changing its scope drops all pending results, including credentials.

use super::{Inviting, Listing, Revoking, Tickets};

/// A node-book name already parsed by the client at the book boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineKey(String);

impl MachineKey {
    /// Carry an existing node-book name into a per-machine request.
    #[must_use]
    pub fn from_book(name: &str) -> Self {
        Self(name.to_owned())
    }
    /// The name used by the connection registry.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The selected machine's availability, without inventing a live-device
/// status the node does not report.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Target {
    /// This page is not shown (pending credentials are discarded).
    #[default]
    Hidden,
    /// The node book has not arrived yet.
    Loading,
    /// The book contains no machine.
    NoMachines,
    /// The selected machine is unavailable.
    Offline(MachineKey),
    /// Requests can be sent through this connection.
    Online(MachineKey),
}

/// State whose lifetime is the selected machine, plus a ticket source
/// whose lifetime is the app: an A-to-B-to-A switch cannot reuse tickets.
#[derive(Debug, Default)]
pub struct Page {
    /// What the page is about.
    pub target: Target,
    /// The device list's state.
    pub listing: Listing,
    /// The in-memory invitation.
    pub inviting: Inviting,
    /// Revocation state, from selection through confirmed response.
    pub revoking: Revoking,
    /// Request identities retained across target changes.
    pub tickets: Tickets,
}

impl Page {
    /// Replace scope, dropping everything belonging to the previous one.
    pub fn target(&mut self, target: Target) {
        if self.target != target {
            self.listing = Listing::Empty;
            self.inviting = Inviting::Empty;
            self.revoking = Revoking::Empty;
            self.target = target;
        }
    }
}
