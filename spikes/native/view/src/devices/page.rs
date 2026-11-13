//! Request identity and page states. A late answer must not put one
//! machine's credential under another's name (or restore a replaced link).

use vessel_proto::{DeviceId, DeviceInfo, ErrorCode};

use super::Pairing;

/// A page request's identity; never reused during the page's lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ticket(u64);

/// A bounded source of identities. Exhaustion refuses a request rather
/// than reusing an old identity and accepting an old answer.
#[derive(Debug, Default)]
pub struct Tickets(u64);

impl Tickets {
    /// Reserve the next identity, or refuse after all identities were used.
    ///
    /// # Errors
    /// All identities have been used; none may be reused.
    pub fn next_ticket(&mut self) -> Result<Ticket, Problem> {
        self.0 = self.0.checked_add(1).ok_or(Problem::Exhausted)?;
        Ok(Ticket(self.0))
    }
}

/// Safe failures: no node-supplied error text can echo a credential into
/// the UI or a log. The link maps codes at the untrusted boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// No live connection.
    #[error("the machine is not connected")]
    Offline,
    /// The node refused the call, with this typed code.
    #[error("the node refused the call ({0:?})")]
    Refused(ErrorCode),
    /// No address in an otherwise valid invitation can reach its node.
    #[error("the machine has no published pairing route")]
    NoRoute,
    /// The response or invitation could not be decoded.
    #[error("the machine returned an invalid response")]
    InvalidReply,
    /// This page cannot safely issue another request.
    #[error("request identities exhausted; reopen the app")]
    Exhausted,
}

impl Problem {
    /// Failures are error notices, independently of short status badges.
    #[must_use]
    pub const fn notice(self) -> crate::alert::Variant {
        match self {
            Self::Offline
            | Self::Refused(_)
            | Self::NoRoute
            | Self::InvalidReply
            | Self::Exhausted => crate::alert::Variant::Error,
        }
    }
}

/// A read, with identity retained until its answer arrives.
#[derive(Debug, Default)]
pub enum Listing {
    /// Not requested for this machine.
    #[default]
    Empty,
    /// A read is in progress.
    Loading(Ticket),
    /// Its devices, including revoked entries for the page's filter.
    Ready(Vec<DeviceInfo>),
    /// A failed read, retryable by the person.
    Failed(Problem),
}

impl Listing {
    /// The semantic signal of this read, shared with the drawing layer.
    #[must_use]
    pub const fn variant(&self) -> crate::badge::Variant {
        match self {
            Self::Empty | Self::Loading(_) => crate::badge::Variant::Muted,
            Self::Ready(_) => crate::badge::Variant::Success,
            Self::Failed(_) => crate::badge::Variant::Error,
        }
    }

    /// Apply only the outstanding read's answer.
    pub fn answer(&mut self, ticket: Ticket, result: Result<Vec<DeviceInfo>, Problem>) {
        if matches!(self, Self::Loading(expected) if *expected == ticket) {
            *self = match result {
                Ok(devices) => Self::Ready(devices),
                Err(why) => Self::Failed(why),
            };
        }
    }
}

/// Making an invitation never leaves the previous credential visible.
#[derive(Debug, Default)]
pub enum Inviting {
    /// No invitation has been made.
    #[default]
    Empty,
    /// A new link is requested.
    Loading(Ticket),
    /// An in-memory, masked-by-default credential.
    Ready(Pairing),
    /// The request failed.
    Failed(Problem),
}

impl Inviting {
    /// The semantic signal of making a link.
    #[must_use]
    pub const fn variant(&self) -> crate::badge::Variant {
        match self {
            Self::Empty | Self::Loading(_) => crate::badge::Variant::Muted,
            Self::Ready(_) => crate::badge::Variant::Success,
            Self::Failed(_) => crate::badge::Variant::Error,
        }
    }

    /// Apply only this request's result, never an earlier invitation.
    /// A true result asks the page to refresh devices that may have paired.
    pub fn answer(&mut self, ticket: Ticket, result: Result<Pairing, Problem>) -> bool {
        if !matches!(self, Self::Loading(expected) if *expected == ticket) {
            return false;
        }
        *self = match result {
            Ok(pairing) => Self::Ready(pairing),
            Err(why) => Self::Failed(why),
        };
        true
    }

    /// Mask a ready credential on copy, focus loss or navigation.
    pub fn hide(&mut self) {
        if let Self::Ready(pairing) = self {
            pairing.hide();
        }
    }
}

/// Revocation requires an explicit confirmation before a call is sent.
#[derive(Debug, Default)]
pub enum Revoking {
    /// Nothing selected.
    #[default]
    Empty,
    /// The kit's confirmation is open for this device.
    Confirm(DeviceInfo),
    /// Confirmed, waiting for the node.
    Sending {
        /// The request identity.
        ticket: Ticket,
        /// Keep the confirmation's public name while its button is disabled.
        device: DeviceInfo,
    },
    /// The call failed, shown next to the revoke controls.
    Failed(Problem),
}

impl Revoking {
    /// The semantic signal of the access-removal operation.
    #[must_use]
    pub const fn variant(&self) -> crate::badge::Variant {
        match self {
            Self::Empty | Self::Confirm(_) | Self::Sending { .. } => crate::badge::Variant::Muted,
            Self::Failed(_) => crate::badge::Variant::Error,
        }
    }

    /// Keep the same confirmation visible through the pending request.
    /// The screen needs no second, potentially stale copy of its name.
    #[must_use]
    pub const fn confirmation(&self) -> Option<&DeviceInfo> {
        match self {
            Self::Confirm(device) | Self::Sending { device, .. } => Some(device),
            Self::Empty | Self::Failed(_) => None,
        }
    }

    /// Confirm once; repeated presses cannot send a second request.
    pub fn confirm(&mut self, ticket: Ticket) -> Option<DeviceId> {
        let Self::Confirm(info) = self else {
            return None;
        };
        let device = info.clone();
        let id = device.id;
        *self = Self::Sending { ticket, device };
        Some(id)
    }

    /// Settle only the outstanding revocation. A success asks for a fresh
    /// list, never removes a row optimistically before the node agrees.
    pub fn answer(&mut self, ticket: Ticket, result: Result<(), Problem>) -> bool {
        if !matches!(self, Self::Sending { ticket: expected, .. } if *expected == ticket) {
            return false;
        }
        *self = match result {
            Ok(()) => Self::Empty,
            Err(why) => Self::Failed(why),
        };
        true
    }
}
