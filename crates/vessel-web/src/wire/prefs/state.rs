//! The bridge publishes each complete preferences state with a revision
//! so a delayed reply cannot replace a newer choice in the browser.

use serde::{Deserialize, Serialize};

use super::{Kept, Preferences};

/// Where the preferences are now, with every value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PreferencesState {
    /// Rises by one with every state the bridge publishes in this launch;
    /// the page drops a reply or a push older than what it shows.
    pub revision: Revision,
    /// Where they are kept.
    pub kept: Kept,
    /// The values.
    pub values: Preferences,
}

/// The number of a published preferences state within one launch, from
/// 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct Revision(u64);

impl Revision {
    /// The first state of a launch.
    pub const FIRST: Self = Self(1);

    /// The state after this one. Saturating: a launch would need 2^64
    /// changes to reach the end, and the page then keeps the last state.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// The number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}
