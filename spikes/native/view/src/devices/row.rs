//! Compact row facts; complete scopes and pairing dates stay in the tooltip.
//!
//! [Decision Log]
//! - Purpose: one access/last-use hint, with badges only for exceptions.
//! - Constraint: a normal active device is already described by the list's
//!   heading. The current caller must never be guessed from its name.
//! - Choice: summarize familiar access in the row, retain every scope in
//!   the exact tooltip, and accept an optional proven caller identity.
//! - Why: no repeated paired badge or three-line history; no invented self.
//! - Cost: caller identity remains unreported until the connection supplies it.
use crate::badge::Variant;
use vessel_proto::{DeviceId, DeviceInfo, Scope};

/// A short access summary, not a replacement for the full scope set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// May manage the machine.
    Manage,
    /// May operate sessions.
    Sessions,
    /// Read is its only scope.
    ReadOnly,
    /// May receive the home's shared settings.
    Sharing,
    /// May pass requests to other machines.
    Forwarding,
    /// No scope was granted.
    None,
}
impl Access {
    /// Keep independent fleet privileges from being mislabeled read-only.
    #[must_use]
    pub fn of(scopes: &[Scope]) -> Self {
        if scopes.contains(&Scope::Admin) {
            Self::Manage
        } else if scopes.contains(&Scope::Operate) {
            Self::Sessions
        } else if scopes.contains(&Scope::Sync) {
            Self::Sharing
        } else if scopes.contains(&Scope::Relay) {
            Self::Forwarding
        } else if scopes.contains(&Scope::Read) {
            Self::ReadOnly
        } else {
            Self::None
        }
    }
}

/// Only facts that distinguish a row from a normal paired device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceBadge {
    /// Proven identity of the device using this connection.
    ThisDevice,
    /// No authentication has been recorded yet.
    Unseen,
    /// Access was removed.
    Revoked,
}
impl DeviceBadge {
    /// `caller` is a verified connection identity, never a matched label.
    #[must_use]
    pub fn of(device: &DeviceInfo, caller: Option<DeviceId>) -> Option<Self> {
        if device.revoked_at.is_some() {
            Some(Self::Revoked)
        } else if caller == Some(device.id) {
            Some(Self::ThisDevice)
        } else if device.last_seen.is_none() {
            Some(Self::Unseen)
        } else {
            None
        }
    }
    /// Shared semantic colour; no ordinary paired row needs a signal.
    #[must_use]
    pub const fn variant(self) -> Variant {
        match self {
            Self::ThisDevice => Variant::Success,
            Self::Unseen => Variant::Muted,
            Self::Revoked => Variant::Error,
        }
    }
}
