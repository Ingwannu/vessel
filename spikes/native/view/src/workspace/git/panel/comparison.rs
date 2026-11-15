//! Comparison identity: UI modes never silently alias a different wire scope.

use serde::{Deserialize, Serialize};
use vessel_proto::NodeId;
use vessel_proto::workspace::{DiffScope, WorkspaceRoot, WorktreeBase};

/// A node I/O target, not a replacement for registry `WorkspaceId` or layout keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RepositoryKey {
    /// The actual node identity; aliases do not share another node's cache.
    pub node: NodeId,
    /// Resolved again by the node before any I/O.
    pub root: WorkspaceRoot,
}

/// Paseo's three comparison choices. Mode selection is ephemeral per workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonMode {
    /// HEAD against working changes, including new files.
    Uncommitted,
    /// Merge base against HEAD, excluding working changes.
    Committed,
    /// Merge base against working changes, including new files.
    #[default]
    All,
}

/// A displayed preference also used in node-query identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Whitespace {
    /// Preserve whitespace-only edits.
    #[default]
    Show,
    /// Ask the node to ignore them.
    Ignore,
}

impl Whitespace {
    /// The legacy wire uses a boolean; convert only at that boundary.
    #[must_use]
    pub const fn ignored(self) -> bool {
        matches!(self, Self::Ignore)
    }
}

/// Normalized query identity: Uncommitted never retains an irrelevant base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    mode: ComparisonMode,
    base: Option<WorktreeBase>,
    whitespace: Whitespace,
}

impl Comparison {
    /// Build a normalized identity, as Paseo's diff query normalizer does.
    #[must_use]
    pub fn new(mode: ComparisonMode, base: Option<WorktreeBase>, whitespace: Whitespace) -> Self {
        let base = match mode {
            ComparisonMode::Uncommitted => None,
            ComparisonMode::Committed | ComparisonMode::All => base,
        };
        Self {
            mode,
            base,
            whitespace,
        }
    }

    /// The selected mode, used by labels and empty-state navigation.
    #[must_use]
    pub const fn mode(&self) -> ComparisonMode {
        self.mode
    }

    /// The retained base; absent means the root's authoritative default.
    #[must_use]
    pub fn base(&self) -> Option<&WorktreeBase> {
        self.base.as_ref()
    }

    /// Whitespace participates in cache and review-draft identity.
    #[must_use]
    pub const fn whitespace(&self) -> Whitespace {
        self.whitespace
    }

    /// Adapt only modes the legacy node actually implements. None means
    /// Committed needs ADR 0087, never that All may be substituted for it.
    #[must_use]
    pub fn legacy_scope(&self) -> Option<DiffScope> {
        match self.mode {
            ComparisonMode::Uncommitted => Some(DiffScope::Uncommitted),
            ComparisonMode::All => Some(DiffScope::Base {
                base: self.base.clone(),
            }),
            ComparisonMode::Committed => None,
        }
    }
}

impl Default for Comparison {
    fn default() -> Self {
        Self::new(ComparisonMode::All, None, Whitespace::Show)
    }
}
