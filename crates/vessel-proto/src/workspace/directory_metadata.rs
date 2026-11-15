//! Additive directory metadata; an old file.list reply is never changed
//! merely because a native client needs modification-time ordering.

use super::{EntryKind, RelPath, WorkspaceRoot, panel_files::FileSize};
use serde::{Deserialize, Serialize};

/// Filesystem modification time for display/order, never a save guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct FileModifiedMillis(pub u64);

/// Whether directory discovery includes git-ignored entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum IgnoredEntries {
    /// Explorer can display ignored entries dimmed.
    Include,
    /// Picker discovery prunes ignored entries.
    Exclude,
}

/// Git's classification, independent from the request's inclusion policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum IgnoreState {
    /// Ignored according to git's actual rules.
    Ignored,
    /// Not ignored, including a non-git folder.
    Visible,
}

/// Directory inspection carries the metadata needed by all three sort modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InspectDirectory {
    /// Authorized root, resolved again by the destination node.
    pub root: WorkspaceRoot,
    /// Directory below that root.
    pub directory: RelPath,
    /// Explicit ignore policy.
    pub ignored: IgnoredEntries,
}

/// A validated entry with optional, honestly unknown metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InspectedEntry {
    /// Full relative path, including its basename.
    pub path: RelPath,
    /// Links remain links, never falsely advertised as regular files.
    pub kind: EntryKind,
    /// Regular-file size when available.
    pub size: Option<FileSize>,
    /// Display/order timestamp; absence is not the Unix epoch.
    pub modified: Option<FileModifiedMillis>,
    /// Actual git classification.
    pub ignored: IgnoreState,
}

/// One bounded directory snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InspectedDirectory {
    /// At most the ordinary file-list budget, directories first.
    pub entries: Vec<InspectedEntry>,
    /// The directory exceeded the response budget.
    pub more: bool,
}
