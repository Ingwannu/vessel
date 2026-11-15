//! Feature-owned pane state and global display preferences, after Paseo.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use vessel_proto::workspace::{CommitId, RelPath};

/// Requested diff layout. Compact surfaces project Split to Unified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// One column retaining both sides' line numbers.
    #[default]
    Unified,
    /// Old and new text in separate columns.
    Split,
}

/// Whether long source lines wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Wrap {
    /// Horizontal scrolling preserves source lines.
    #[default]
    NoWrap,
    /// Long source lines use additional visual rows.
    Wrap,
}

/// Whether a desktop file tree accompanies the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreeVisibility {
    /// Show the document without a desktop tree rail.
    #[default]
    Hidden,
    /// Show a resizable desktop tree rail.
    Visible,
}

/// Opening a file from Changes opens a diff tab unless inline is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InlineDiff {
    /// A Changes file opens a separate working-diff target.
    #[default]
    Separate,
    /// Changes includes each file's diff body.
    Inline,
}

/// Folded section state, shared by commits demand and its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fold {
    /// Descendant content does not demand new reads.
    #[default]
    Closed,
    /// Descendant content may demand reads while active.
    Open,
}

/// Global preferences; comparison mode deliberately does not persist here.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DisplayPreferences {
    /// Unified by default.
    pub layout: Layout,
    /// No desktop rail until requested.
    pub tree: TreeVisibility,
    /// Preserve horizontal source geometry by default.
    pub wrap: Wrap,
    /// Show whitespace changes by default.
    pub whitespace: super::Whitespace,
    /// File activation opens a separate working diff by default.
    pub inline: InlineDiff,
    /// The expensive history read starts only after expansion.
    pub commits: Fold,
}

/// State owned by one pane. Its key is supplied by the shared layout owner.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PanelState {
    /// Individual file bodies folded by the person.
    pub collapsed_files: BTreeSet<RelPath>,
    /// Diff-folder identities, distinct from filesystem explorer state.
    pub collapsed_folders: BTreeSet<RelPath>,
    /// History demand is local to this panel.
    pub commits: Fold,
}

/// Git content identity. W12 adapts its shared Target rather than copying it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelTarget {
    /// Explorer Changes target.
    Changes,
    /// A working-diff document in a pane.
    WorkingDiff,
    /// An immutable commit document, without working-tree file actions.
    CommitDiff(CommitId),
}

/// User capability, not a toolkit/platform-name test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormFactor {
    /// Desktop pane with pointer-oriented document capabilities.
    Desktop,
    /// Narrow pane or touch surface with combined presentation.
    Compact,
}

/// Which existing renderer to use for the target and preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presentation {
    /// Changed-file hierarchy without inline bodies.
    Tree,
    /// Scrolling document with optional desktop tree rail.
    Diff,
    /// File headers and diff bodies together in Changes.
    Combined,
}

impl Presentation {
    /// Paseo's tree target becomes combined in compact/inline presentation.
    #[must_use]
    pub const fn for_target(target: &PanelTarget, inline: InlineDiff, form: FormFactor) -> Self {
        match (target, inline, form) {
            (PanelTarget::Changes, InlineDiff::Separate, FormFactor::Desktop) => Self::Tree,
            (PanelTarget::Changes, InlineDiff::Inline, FormFactor::Desktop)
            | (
                PanelTarget::Changes,
                InlineDiff::Separate | InlineDiff::Inline,
                FormFactor::Compact,
            ) => Self::Combined,
            (
                PanelTarget::WorkingDiff | PanelTarget::CommitDiff(_),
                InlineDiff::Separate | InlineDiff::Inline,
                FormFactor::Desktop | FormFactor::Compact,
            ) => Self::Diff,
        }
    }
}

impl Layout {
    /// A narrow pane cannot render side-by-side columns legibly.
    #[must_use]
    pub const fn effective(self, form: FormFactor) -> Self {
        match form {
            FormFactor::Desktop => self,
            FormFactor::Compact => Self::Unified,
        }
    }
}
