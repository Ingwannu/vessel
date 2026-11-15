//! Correlation and typed failures for schedule calls. Identities never wrap;
//! replies from an earlier visit cannot settle a newer read or mutation.

use std::sync::Arc;
use vessel_proto::WireError;

/// Concurrent Automation operations across the aggregate page and its link.
pub const MAX_IN_FLIGHT: usize = 4;

/// A machine's validated book key, separate from its display/host name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MachineKey(Arc<str>);

impl MachineKey {
    /// Parse once at the fleet boundary. The view cannot depend on the client's
    /// book module, so reuse its existing view-side name contract.
    #[must_use]
    pub fn of(name: &str) -> Option<Self> {
        crate::machines::name_problem(name, &[], None)
            .is_none()
            .then(|| Self(Arc::from(name)))
    }

    /// Reuse the canonical parsed book key without validating its text twice.
    pub(crate) fn admitted(context: &crate::connection::ConnectionContext) -> Self {
        // The canonical context already carries the parsed book key. Do not
        // parse that alias again or derive it from a node display name.
        Self(Arc::from(context.machine().as_str()))
    }

    /// The key handed back to the link's book lookup.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for MachineKey {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

/// One local request identity, with no meaning on the node's wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ticket(u64);

/// Monotonic identities issued by one screen model.
#[derive(Debug, Default)]
pub struct Tickets(u64);

impl Tickets {
    /// Refuse a request on exhaustion rather than reusing a delayed identity.
    pub fn issue(&mut self) -> Option<Ticket> {
        self.0 = self.0.checked_add(1)?;
        Some(Ticket(self.0))
    }
}

/// A failure the pure view can interpret; link errors retain their full source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// Actual node refusal, with its typed code and original diagnostic data.
    Node(WireError),
    /// Automation methods are absent from this node's protocol.
    Older,
    /// This build has not enabled the requested feature; no call was sent.
    NotEnabled,
    /// Machine is not connected.
    Offline,
    /// The originally selected physical admission was replaced before dispatch.
    ConnectionChanged,
    /// A safe read exceeded the operation deadline.
    TimedOut,
    /// Bounded request pool is full.
    Busy,
    /// A typed response named a resource other than the requested one.
    Unexpected,
    /// A write lost its reply; the machine may already have acted.
    Unconfirmed,
    /// This browser build has no node transport.
    NoTransport,
    /// Local selection preferences failed to persist; no schedule RPC was sent.
    Preferences,
    /// Local identities exhausted; reopening the app gives a fresh model.
    Exhausted,
}

impl Failure {
    /// Reads are safe to repeat; these failures need an external fix or reconciliation first.
    #[must_use]
    pub const fn retryable(&self) -> bool {
        match self {
            Self::Older
            | Self::NotEnabled
            | Self::Exhausted
            | Self::Unconfirmed
            | Self::NoTransport
            | Self::Preferences => false,
            Self::Node(_)
            | Self::Offline
            | Self::ConnectionChanged
            | Self::TimedOut
            | Self::Busy
            | Self::Unexpected => true,
        }
    }
    /// Ordinary offline reads use the reference's load-error sentence. Typed
    /// refusals and local limits keep the corrective words a retry alone lacks.
    #[must_use]
    pub const fn needs_explanation(&self) -> bool {
        match self {
            Self::Node(_)
            | Self::ConnectionChanged
            | Self::TimedOut
            | Self::Older
            | Self::NotEnabled
            | Self::Busy
            | Self::Unexpected
            | Self::Exhausted
            | Self::Unconfirmed
            | Self::NoTransport
            | Self::Preferences => true,
            Self::Offline => false,
        }
    }
    /// Shared Alert meaning; a caller never chooses its own warning colour.
    #[must_use]
    pub const fn variant(&self) -> crate::alert::Variant {
        match self {
            Self::Older | Self::NotEnabled | Self::NoTransport | Self::ConnectionChanged => {
                crate::alert::Variant::Warning
            }
            Self::Busy => crate::alert::Variant::Info,
            Self::Node(_)
            | Self::Offline
            | Self::TimedOut
            | Self::Unexpected
            | Self::Exhausted
            | Self::Unconfirmed
            | Self::Preferences => crate::alert::Variant::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_keys_and_tickets_have_no_ambiguous_fallback() {
        for name in ["laptop", "build-server", "box.2"] {
            assert_eq!(MachineKey::of(name).unwrap().as_str(), name);
        }
        for name in ["", "bad name", "../node", "한글", "-node"] {
            assert!(MachineKey::of(name).is_none());
        }
        let mut tickets = Tickets(u64::MAX.saturating_sub(1));
        assert!(tickets.issue().is_some());
        assert_eq!(tickets.issue(), None);
        assert_eq!(tickets.issue(), None);
    }
}
