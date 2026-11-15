//! Modifier state is finite, with no duplicated names or unbounded array retention.

use serde::{Deserialize, Serialize};

use super::KeyModifier;

/// A fixed keyboard state shared by tool clicks and direct human input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct KeyModifiers {
    /// Alternate key held.
    #[serde(default)]
    pub alt: bool,
    /// Control key held.
    #[serde(default)]
    pub control: bool,
    /// Meta key held.
    #[serde(default)]
    pub meta: bool,
    /// Shift key held.
    #[serde(default)]
    pub shift: bool,
}
impl KeyModifiers {
    /// MCP's array edge folds names once into fixed state; repeats add no authority.
    pub fn include(&mut self, modifier: KeyModifier) {
        match modifier {
            KeyModifier::Alt => self.alt = true,
            KeyModifier::Control => self.control = true,
            KeyModifier::Meta => self.meta = true,
            KeyModifier::Shift => self.shift = true,
        }
    }
}
