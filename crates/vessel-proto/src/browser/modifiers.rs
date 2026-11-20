//! Modifier state is finite, with no duplicated names or unbounded array retention.

use serde::{Deserialize, Serialize};

use super::KeyModifier;

/// A held modifier is a fact, distinct from an incoming key-down/up event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ModifierState {
    /// The key contributes no modifier to input dispatch.
    #[default]
    Released,
    /// The key is currently held during this input event.
    Held,
}

/// A fixed keyboard state shared by tool clicks and direct human input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct KeyModifiers {
    /// Alternate key held.
    #[serde(default)]
    pub alt: ModifierState,
    /// Control key held.
    #[serde(default)]
    pub control: ModifierState,
    /// Meta key held.
    #[serde(default)]
    pub meta: ModifierState,
    /// Shift key held.
    #[serde(default)]
    pub shift: ModifierState,
}
impl KeyModifiers {
    /// MCP's array edge folds names once into fixed state; repeats add no authority.
    pub fn include(&mut self, modifier: KeyModifier) {
        match modifier {
            KeyModifier::Alt => self.alt = ModifierState::Held,
            KeyModifier::Control => self.control = ModifierState::Held,
            KeyModifier::Meta => self.meta = ModifierState::Held,
            KeyModifier::Shift => self.shift = ModifierState::Held,
        }
    }
}
