//! The default-off agent switch does not grant authority to the human control API.

use serde::{Deserialize, Serialize};

/// Unknown saved values stay disabled; only an explicit supported On can offer tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum BrowserTools {
    /// Fresh installs and older settings files offer no browser tools.
    #[default]
    Off,
    /// Still requires a qualified engine, Operate and the session-start admission.
    On,
    /// A later setting cannot silently turn browser access on after downgrade.
    #[serde(other)]
    Unknown,
}
impl BrowserTools {
    /// Keep an absent/default field compatible with older saved files and replies.
    #[must_use]
    pub fn is_off(&self) -> bool {
        !matches!(self, Self::On)
    }
}
