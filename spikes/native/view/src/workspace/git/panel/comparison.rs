//! Comparison identity: UI modes never silently alias a different wire scope.

use serde::{Deserialize, Serialize};
use vessel_proto::NodeId;
use vessel_proto::workspace::{
    CommitId, CompareGit, DiffScope, GitComparison, GitReadReason, GitWhitespace, WorkspaceRoot,
    WorktreeBase,
};

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

/// A commit document cannot inherit a branch comparison's mutable base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComparisonKind {
    /// One of the three working-checkout comparisons.
    Branch(ComparisonMode),
    /// One fixed commit against its first parent, or the empty tree.
    Commit(CommitId),
}

/// Normalized query identity: Uncommitted never retains an irrelevant base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    kind: ComparisonKind,
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
            kind: ComparisonKind::Branch(mode),
            base,
            whitespace,
        }
    }

    /// The selected mode, used by labels and empty-state navigation.
    #[must_use]
    pub const fn mode(&self) -> Option<ComparisonMode> {
        match &self.kind {
            ComparisonKind::Branch(mode) => Some(*mode),
            ComparisonKind::Commit(_) => None,
        }
    }

    /// Immutable commit identity remains part of every summary/body ticket.
    #[must_use]
    pub const fn kind(&self) -> &ComparisonKind {
        &self.kind
    }

    /// A commit document has no mutable branch base to accidentally reuse.
    #[must_use]
    pub const fn at_commit(id: CommitId, whitespace: Whitespace) -> Self {
        Self {
            kind: ComparisonKind::Commit(id),
            base: None,
            whitespace,
        }
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

    /// The canonical staged call carries normalized mode/base/whitespace.
    /// Admission still belongs to the client's negotiated method, not this DTO.
    #[must_use]
    pub fn query(&self, root: WorkspaceRoot, reason: GitReadReason) -> CompareGit {
        let comparison = match &self.kind {
            ComparisonKind::Branch(ComparisonMode::Uncommitted) => GitComparison::Uncommitted,
            ComparisonKind::Branch(ComparisonMode::Committed) => GitComparison::Committed {
                base: self.base.clone(),
            },
            ComparisonKind::Branch(ComparisonMode::All) => GitComparison::All {
                base: self.base.clone(),
            },
            ComparisonKind::Commit(id) => GitComparison::Commit { id: id.clone() },
        };
        let whitespace = match self.whitespace {
            Whitespace::Show => GitWhitespace::Show,
            Whitespace::Ignore => GitWhitespace::Ignore,
        };
        CompareGit {
            root,
            comparison,
            whitespace,
            reason,
        }
    }

    /// Adapt only exact legacy meanings. All needs ADR 0087's private-index
    /// read (legacy Base can omit restored untracked paths); Committed also
    /// uses the canonical contract. Neither is approximated by a different scope.
    #[must_use]
    pub fn legacy_scope(&self) -> Option<DiffScope> {
        match &self.kind {
            ComparisonKind::Branch(ComparisonMode::Uncommitted) => Some(DiffScope::Uncommitted),
            ComparisonKind::Branch(ComparisonMode::All | ComparisonMode::Committed) => None,
            ComparisonKind::Commit(id) => Some(DiffScope::Commit { id: id.clone() }),
        }
    }
}

impl Default for Comparison {
    fn default() -> Self {
        Self::new(ComparisonMode::All, None, Whitespace::Show)
    }
}
