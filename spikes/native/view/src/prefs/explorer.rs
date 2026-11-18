//! The Files tree's order, kept with the layout (`Layout::explorer_sort`;
//! split from `mod.rs` for the file budget, docs/QUALITY.md M3).

use serde::{Deserialize, Serialize};

/// The order of a folder's entries in the Files tree: folders first, then
/// by name, by last change (newest first) or by size (largest first), as
/// Paseo's file explorer sorts (the bridge's `ExplorerSort`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplorerSort {
    /// By name.
    #[default]
    Name,
    /// By last change, newest first.
    Modified,
    /// By size, largest first.
    Size,
}

impl ExplorerSort {
    /// The order after this one in Paseo's cycle: Name, Modified, Size.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Name => Self::Modified,
            Self::Modified => Self::Size,
            Self::Size => Self::Name,
        }
    }
}
