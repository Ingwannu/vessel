//! Device-scoped, account-free Web Push (ADR 0041). Endpoints and auth
//! values are capabilities, so Debug never prints them, including in a
//! failed frame decode. Payloads contain no session or request content.
mod material;
use crate::SessionId;
use chrono::{DateTime, Utc};
pub use material::{InvalidPushMaterial, PushAuth, PushEndpoint, PushPublicKey};
use serde::{Deserialize, Serialize};

/// A browser's subscription, owned by the authenticated paired device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PushSubscription {
    /// HTTPS service capability, never a URL to navigate to or log.
    pub endpoint: PushEndpoint,
    /// Device's uncompressed P-256 encryption public key.
    pub p256dh: PushPublicKey,
    /// Device's authentication secret for payload key derivation.
    pub auth: PushAuth,
}
/// Get the node's public VAPID identity, never its private key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetPushKey;
/// The public identity passed to the browser's pushManager.subscribe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PushKey {
    /// Uncompressed P-256 point, base64url without padding.
    pub key: PushPublicKey,
}
/// Replace or renew this paired device's single subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SubscribePush {
    /// Browser subscription, with no caller-supplied device identity.
    pub subscription: PushSubscription,
}
/// Confirmation of the node's lease. Renew by subscribing again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PushLease {
    /// Subscription stops receiving after this time, even across restart.
    pub expires_at: DateTime<Utc>,
}
/// Stop push to this paired device alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UnsubscribePush;

/// What the person is looking at, without disclosing any content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "focus", rename_all = "snake_case")]
pub enum PushFocus {
    /// Visible and focused on this conversation.
    Session {
        /// Session this device is viewing.
        session: SessionId,
    },
    /// Visible and focused on an independent terminal (ADR 0089).
    Terminal {
        /// Exact terminal this connection is viewing.
        terminal: crate::TerminalId,
    },
    /// Visible, focused on another screen.
    Elsewhere,
    /// Hidden or unfocused.
    Background,
}
/// Whether this report corresponds to a person's input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PushActivity {
    /// Keyboard, pointer or touch activity; timestamped by the node.
    Active,
    /// Only visibility/focus changed; do not extend the activity lease.
    Unchanged,
}
/// OS-observed elapsed idle milliseconds, never a client wall-clock timestamp.
/// The node subtracts it from its own now; a report cannot name future activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct PushIdleAge(u32);
impl PushIdleAge {
    /// Wrap an operating system's nonnegative idle counter.
    #[must_use]
    pub const fn new(milliseconds: u32) -> Self {
        Self(milliseconds)
    }
    /// Elapsed idle milliseconds.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}
/// Presence for this connection: a socket alone never suppresses push.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReportPushPresence {
    /// Current visible target.
    pub focus: PushFocus,
    /// A real input event, or only a focus change.
    pub activity: PushActivity,
    /// Optional desktop OS evidence; absent on older/native backends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle_age: Option<PushIdleAge>,
    /// Whether this window tells notices as the system's notifications.
    /// Only a notifying window can be elected a claim's local recipient,
    /// as only Paseo's clients subscribed with `notifications: true` are
    /// (`websocket-server.ts:2884-2886`). Suppression and push still count
    /// every present window. Absent reads `true`, the meaning before the
    /// field; `false` is versioned to ADR 0089's protocol.
    #[serde(default = "notifies", skip_serializing_if = "is_notifying")]
    pub notifies: bool,
}

/// `notifies` when a report leaves it out.
const fn notifies() -> bool {
    true
}

/// Whether `notifies` is its default, which is left off the wire.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
const fn is_notifying(notifies: &bool) -> bool {
    *notifies
}

#[cfg(test)]
mod tests;
