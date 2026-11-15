//! Local selection and deduplication over the plugin owner's eligible themes.
//! No runtime, registry, transport, colour parser or executable plugin content.
//!
//! [Decision Log]
//! - Purpose: retain an unavailable plugin choice while resolving System.
//! - Constraint: preserve the existing eight built-in Theme API and one saved
//!   preference store; runtime generations/eligibility belong to plugins.
//! - Choice: typed Selected/Remembered plugin key plus last built-in choice.
//!   Preferred hosts are transient, bounded navigation hints, never authority.
//! - Why: malformed/half-selected identities cannot reach the palette adapter;
//!   choosing a built-in preserves the remembered key without selecting it.
//! - Impact: native file representation differs from Paseo's two loose fields;
//!   effective selection, fallback, ordering and host preference are equivalent.
mod catalog;
mod key;
mod preferred;

pub use catalog::{Candidate, Catalog, Entry, Resolution};
pub use key::{InvalidThemeKey, PluginThemeId};
pub use preferred::PreferredHosts;
use serde::{Deserialize, Serialize};

/// A remembered key is independent of which built-in was chosen last.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "theme", rename_all = "snake_case")]
pub enum ContributedChoice {
    /// Resolve the key if available, otherwise System, retaining this choice.
    Selected(PluginThemeId),
    /// A built-in is active; keep this key for a future plugin selection.
    Remembered(PluginThemeId),
}
