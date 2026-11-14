//! A badge is a short fact about credential state, never a guessed live
//! connection. Its explanation and exact timestamps stay with the row.

use super::{Access, time::Age};
use crate::{Lang, badge::Variant};
use chrono::{DateTime, FixedOffset, Utc};
use vessel_proto::DeviceInfo;

/// The node reports authentication history, not current connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// The credential has been used and still grants access.
    Paired,
    /// Paired but not authenticated yet.
    Unseen,
    /// The node removed access.
    Revoked,
}

impl DeviceState {
    /// Interpret only the fields the node actually reports.
    #[must_use]
    pub const fn of(device: &DeviceInfo) -> Self {
        if device.revoked_at.is_some() {
            Self::Revoked
        } else if device.last_seen.is_none() {
            Self::Unseen
        } else {
            Self::Paired
        }
    }
    /// The shared badge meaning; the app maps it to the kit once.
    #[must_use]
    pub const fn variant(self) -> Variant {
        match self {
            Self::Paired => Variant::Success,
            Self::Unseen => Variant::Muted,
            Self::Revoked => Variant::Error,
        }
    }
}

/// The row's single compact access/last-use hint. Pairing history and
/// full scope facts belong to the exact tooltip, not extra hint lines.
#[must_use]
pub fn details(lang: Lang, device: &DeviceInfo, now: DateTime<Utc>, offset: FixedOffset) -> String {
    lang.devices().row_hint(
        Access::of(&device.scopes),
        device.last_seen.map(|seen| Age::of(seen, now, offset)),
    )
}

/// Exact instants and every scope, for the kit's hover tooltip. Missing is said
/// plainly; an invented date is never substituted for it.
#[must_use]
pub fn exact(lang: Lang, device: &DeviceInfo, offset: FixedOffset) -> String {
    let words = lang.devices();
    words.device_tooltip(
        &words.exact_times(device.created_at, device.last_seen, offset),
        &device.scopes,
    )
}
