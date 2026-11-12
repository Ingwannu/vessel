//! Webhooks (protocol 11, ADR 0034): a door through which something
//! outside the machine starts a schedule's kind of run.
//!
//! [Decision Log]
//! - Purpose: GOAT's webhook-triggered tasks (the one reference with
//!   them): a request from a CI system, a forge or a monitor starts an
//!   agent with the request as data.
//! - A hook is its own thing, not a schedule's cadence: making one opens
//!   a way in, so it needs `admin` (a schedule needs `operate`), and its
//!   log is of deliveries, not of due times.
//! - What reaches the agent, how a request is checked and what is kept is
//!   ADR 0034's; these are the wire's shapes.

use std::fmt;
use std::str::FromStr;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::run::RunOutcome;
use super::schedule::ScheduleTarget;
use super::text::{DeliveryKey, PromptText, ScheduleName};
use crate::id::HookId;
use crate::profile::Creator;

/// A hook's secret: 256 random bits, made by the node, returned by
/// `hook.create` and `hook.rotate` only. Written `whsec_<base64>`,
/// Standard Webhooks' form: the sender signs each request with the
/// decoded bytes (HMAC-SHA256), not with the text.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct HookSecret([u8; 32]);

impl HookSecret {
    /// The text's prefix.
    pub const PREFIX: &'static str = "whsec_";

    /// Wrap secret bytes (made by the node from the OS's randomness).
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The key. Callers sign with it or compare in constant time; they
    /// never log it.
    #[must_use]
    pub const fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for HookSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("HookSecret(<redacted>)")
    }
}

impl fmt::Display for HookSecret {
    /// The text a person gives the sender. Only for that; `Debug` is what
    /// logs see.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, STANDARD.encode(self.0))
    }
}

/// A hook secret's text that is not `whsec_` and 32 bytes of base64.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidHookSecret;

impl fmt::Display for InvalidHookSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a hook secret is `whsec_` and 32 bytes in base64")
    }
}

impl std::error::Error for InvalidHookSecret {}

impl FromStr for HookSecret {
    type Err = InvalidHookSecret;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let encoded = text
            .trim()
            .strip_prefix(Self::PREFIX)
            .ok_or(InvalidHookSecret)?;
        let bytes = STANDARD.decode(encoded).map_err(|_bad| InvalidHookSecret)?;
        <[u8; 32]>::try_from(bytes)
            .map(Self)
            .map_err(|_length| InvalidHookSecret)
    }
}

crate::key::serde_via_text!(HookSecret);

/// A hook as a person writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HookDefinition {
    /// What it is called.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<ScheduleName>,
    /// Where its prompt goes.
    pub target: ScheduleTarget,
    /// What it sends, before the request's body.
    pub prompt: PromptText,
    /// Whether it takes requests.
    pub enabled: bool,
}

/// A hook as the node keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HookInfo {
    /// Its id.
    pub id: HookId,
    /// What it does.
    pub definition: HookDefinition,
    /// Its path on the node's hook listener (`/hooks/<id>`).
    pub path: String,
    /// Who last wrote its definition; its runs act as them.
    pub defined_by: Creator,
    /// When it was made.
    pub created_at: DateTime<Utc>,
    /// Changes at every change.
    pub revision: u32,
}

impl HookInfo {
    /// The most hooks a node keeps.
    pub const MAX: usize = 64;
}

/// A hook just made, or its secret just changed: the only time the secret
/// is sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HookCreated {
    /// The hook.
    pub hook: HookInfo,
    /// Its secret.
    pub secret: HookSecret,
}

/// Whether the node takes hook requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum HookListener {
    /// The node was not started with a hook listener.
    Off,
    /// It listens.
    Listening {
        /// Where (`127.0.0.1:7078`).
        address: String,
    },
    /// It was asked to listen and could not.
    Failed {
        /// Why.
        message: String,
    },
    /// A state a newer node has.
    #[serde(other)]
    Unknown,
}

/// Why a request was refused before anything ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum Rejection {
    /// The signature was missing or wrong.
    Signature,
    /// The timestamp was missing or more than five minutes off.
    Stale,
    /// The body was over 64 KiB.
    TooLarge,
    /// A reason a newer node has.
    #[serde(other)]
    Unknown,
}

/// What became of a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum DeliveryOutcome {
    /// It ran: what the run did, followed as a schedule's run is.
    Ran {
        /// The run's outcome.
        run: RunOutcome,
    },
    /// Its id was seen in the last 24 hours; nothing ran.
    Duplicate,
    /// The target had no room (4 live sessions, or a full queue); nothing
    /// ran, and the id was not remembered, so a retry can run.
    Busy,
    /// Over the hook's 30 a minute; nothing ran.
    RateLimited,
    /// Refused before anything ran.
    Rejected {
        /// Why.
        why: Rejection,
    },
    /// An outcome a newer node has.
    #[serde(other)]
    Unknown,
}

/// One request to a hook, without its body or headers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HookDelivery {
    /// When it came.
    pub at: DateTime<Utc>,
    /// The sender's `webhook-id`, once the signature held.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<DeliveryKey>,
    /// The body's size in bytes (as far as it was read).
    pub bytes: u64,
    /// What became of it.
    pub outcome: DeliveryOutcome,
}

/// `hook.list`: the hooks, and whether the node listens. Needs `read`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListHooks {}

/// Reply to `hook.list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HookList {
    /// Whether the node takes requests.
    pub listener: HookListener,
    /// The hooks, newest first.
    pub hooks: Vec<HookInfo>,
}

/// `hook.create`: make a hook. Needs `admin`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateHook {
    /// What it does.
    pub definition: HookDefinition,
}

/// `hook.update`: replace a hook's definition. Needs `admin`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UpdateHook {
    /// Which.
    pub hook: HookId,
    /// The revision the change was made from; another is `conflict`.
    pub revision: u32,
    /// The new definition.
    pub definition: HookDefinition,
}

/// `hook.rotate`: give a hook a new secret; the old one is refused at
/// once. Needs `admin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RotateHook {
    /// Which.
    pub hook: HookId,
}

/// `hook.delete`: remove a hook and its log. Needs `admin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DeleteHook {
    /// Which.
    pub hook: HookId,
}

/// `hook.deliveries`: a hook's newest deliveries. Needs `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListDeliveries {
    /// Which hook.
    pub hook: HookId,
}

/// Reply to `hook.deliveries`: at most [`DeliveryList::MAX`], newest
/// first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DeliveryList {
    /// The deliveries.
    pub deliveries: Vec<HookDelivery>,
}

impl DeliveryList {
    /// The deliveries kept per hook.
    pub const MAX: usize = 100;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_is_standard_webhooks_text_and_never_debug_printed() {
        let secret = HookSecret::from_bytes([7; 32]);
        let text = secret.to_string();
        assert!(text.starts_with("whsec_"), "{text}");
        assert_eq!(text.parse::<HookSecret>().unwrap(), secret);
        assert_eq!(format!("{secret:?}"), "HookSecret(<redacted>)");
        assert!("whsec_AAAA".parse::<HookSecret>().is_err());
        assert!(STANDARD.encode([7; 32]).parse::<HookSecret>().is_err());
    }

    #[test]
    fn deliveries_have_their_wire_names() {
        let busy = HookDelivery {
            at: DateTime::from_timestamp(1, 0).unwrap(),
            key: None,
            bytes: 12,
            outcome: DeliveryOutcome::Busy,
        };
        let json = serde_json::to_string(&busy).unwrap();
        assert_eq!(
            json,
            r#"{"at":"1970-01-01T00:00:01Z","bytes":12,"outcome":{"outcome":"busy"}}"#
        );
        let newer: DeliveryOutcome = serde_json::from_str(r#"{"outcome":"queued"}"#).unwrap();
        assert_eq!(newer, DeliveryOutcome::Unknown);
    }
}
