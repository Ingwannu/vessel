//! A node's settings that a client reads and changes (protocol 12): for now
//! how long an idle agent runs before it is parked (ADR 0038).
//!
//! [Decision Log]
//! - Purpose: parking was set only by `vesseld --park-after`, so a person
//!   in the app could neither see nor change it. Paseo has a card in a
//!   host's settings that turns idle cleanup on and off, kept by the
//!   daemon and applied at once.
//! - Choice: one settings object, read whole (`node.settings`, read) and
//!   changed field by field (`node.set_settings`, admin), so a later
//!   setting is one more optional field and an older client keeps working.
//!   The node keeps the same type as its file, so what a client sees is
//!   what survives a restart.

use serde::{Deserialize, Serialize};

use crate::parking::ParkAfter;

/// The node's settings in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct NodeSettings {
    /// How long a structured session may stay idle before its agent is
    /// parked.
    pub park_after: ParkAfter,
}

/// `node.settings`: the node's settings in force. Needs `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetNodeSettings {}

/// `node.set_settings`: change the settings it names; the others stay.
/// Needs `admin`. Answers the settings in force.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SetNodeSettings {
    /// How long a structured session may stay idle before its agent is
    /// parked; `unknown` is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub park_after: Option<ParkAfter>,
}
