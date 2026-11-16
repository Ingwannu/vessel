//! Request identity and page states. A late answer must not put one
//! machine's credential under another's name (or restore a replaced link).
//!
//! [Decision Log]
//! - Purpose: keep confirmation intent attached to its physical target.
//! - Constraint: cancel/reopen and same-node reconnect can reuse a device id.
//! - Alternatives: store an optional intent beside a mutable selection.
//! - Choice: one private-field record holds context, intent and device, and
//!   travels unchanged into Sending. The guarded factory owns its creation.
//! - Why: even an accidentally retained record cannot confirm on another node.
//! - Cost: the screen reads its public metadata through getters, not fields.

use vessel_proto::{DeviceId, DeviceInfo, ErrorCode};

use super::{ConnectionContext, Pairing, RevokeIntent};

/// A page request's identity; never reused during the page's lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ticket(u64);

impl std::fmt::Display for Ticket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

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
    /// A callback or queued request belongs to a retired admission.
    #[error("the selected machine connection changed")]
    TargetChanged,
    /// No current node-reported management grant is available to this control.
    #[error("management access is not confirmed for this connection")]
    AccessNotConfirmed,
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
            Self::TargetChanged
            | Self::AccessNotConfirmed
            | Self::Offline
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

/// An immutable confirmed target. Keeping context, intent and device together
/// prevents an old selection being reattached to a replacement admission.
#[derive(Debug)]
pub struct Confirmation {
    pub(super) context: ConnectionContext,
    pub(super) intent: RevokeIntent,
    pub(super) device: DeviceInfo,
}

/// Revocation requires an explicit confirmation before a call is sent.
#[derive(Debug, Default)]
pub enum Revoking {
    /// Nothing selected.
    #[default]
    Empty,
    /// The kit's confirmation is open for this device.
    Confirm(Confirmation),
    /// Confirmed, waiting for the node.
    Sending {
        /// The request identity.
        ticket: Ticket,
        /// Keep the confirmation's public name while its button is disabled.
        confirmation: Confirmation,
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
            Self::Confirm(confirmation) | Self::Sending { confirmation, .. } => {
                Some(&confirmation.device)
            }
            Self::Empty | Self::Failed(_) => None,
        }
    }

    /// The admission the immutable selection came from, including while sending.
    #[must_use]
    pub const fn context(&self) -> Option<&ConnectionContext> {
        match self {
            Self::Confirm(confirmation) | Self::Sending { confirmation, .. } => {
                Some(&confirmation.context)
            }
            Self::Empty | Self::Failed(_) => None,
        }
    }
    /// The intent cannot exist independently of its selected device/context.
    #[must_use]
    pub const fn intent(&self) -> Option<RevokeIntent> {
        match self {
            Self::Confirm(confirmation) | Self::Sending { confirmation, .. } => {
                Some(confirmation.intent)
            }
            Self::Empty | Self::Failed(_) => None,
        }
    }
    /// Confirm once, on the exact admission and intent the button captured.
    pub fn confirm(
        &mut self,
        context: &ConnectionContext,
        intent: RevokeIntent,
        ticket: Ticket,
    ) -> Option<DeviceId> {
        let Self::Confirm(selected) = self else {
            return None;
        };
        if selected.context != *context || selected.intent != intent {
            return None;
        }
        let confirmation = Confirmation {
            context: selected.context.clone(),
            intent: selected.intent,
            device: selected.device.clone(),
        };
        let id = confirmation.device.id;
        *self = Self::Sending {
            ticket,
            confirmation,
        };
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
