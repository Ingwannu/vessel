//! Explorer preferences and path rewriting, independent of the UI and
//! disk. Sorting keeps directories first and preserves listing indices.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use vessel_proto::workspace::{DirEntry, EntryKind, RelPath};

/// Reference file ordering; modified order is enabled with node metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplorerSort {
    /// Case-folded names in deterministic Unicode order.
    #[default]
    Name,
    /// Largest files first; unknown sizes remain last.
    Size,
}

/// Explicit visibility avoids ambiguous boolean parameters at this seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HiddenFiles {
    /// Paseo's default.
    #[default]
    Show,
    /// Dot-prefixed names are excluded from discovery.
    Hide,
}

/// The person's global tree choices (Paseo's panel store).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExplorerOptions {
    /// Directory-first ordering.
    pub sort: ExplorerSort,
    /// Whether dot-prefixed entries are visible.
    pub hidden: HiddenFiles,
}

impl ExplorerOptions {
    /// Entry filtering is separate from exact-path retrieval.
    #[must_use]
    pub fn visible(self, entry: &DirEntry) -> bool {
        self.hidden == HiddenFiles::Show || !entry.name.starts_with('.')
    }

    /// Stable directory-first ordering with deterministic tie breaks.
    #[must_use]
    pub fn compare(self, left: &DirEntry, right: &DirEntry) -> Ordering {
        (left.kind != EntryKind::Dir)
            .cmp(&(right.kind != EntryKind::Dir))
            .then_with(|| match self.sort {
                ExplorerSort::Name => Ordering::Equal,
                ExplorerSort::Size => right.size.cmp(&left.size),
            })
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
    }
}

/// Replace complete path components only, preserving similarly named siblings.
#[must_use]
pub fn replace_prefix(path: &RelPath, from: &RelPath, to: &RelPath) -> Option<RelPath> {
    if path == from {
        return Some(to.clone());
    }
    let suffix = path
        .as_str()
        .strip_prefix(from.as_str())?
        .strip_prefix('/')?;
    suffix
        .split('/')
        .try_fold(to.clone(), |parent, part| parent.child(part).ok())
}

/// Whether revealing a path must also enable hidden entries.
#[must_use]
pub fn hidden_path(path: &RelPath) -> bool {
    path.as_str()
        .split('/')
        .any(|segment| segment.starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, kind: EntryKind, size: Option<u64>) -> DirEntry {
        DirEntry {
            name: name.into(),
            kind,
            size,
            ignored: false,
        }
    }

    #[test]
    fn reference_defaults_and_size_order_preserve_directories_and_unknowns() {
        let options = ExplorerOptions::default();
        assert!(options.visible(&entry(".env", EntryKind::File, None)));
        let options = ExplorerOptions {
            sort: ExplorerSort::Size,
            hidden: HiddenFiles::Hide,
        };
        assert!(!options.visible(&entry(".env", EntryKind::File, None)));
        let mut entries = [
            entry("unknown", EntryKind::File, None),
            entry("small", EntryKind::File, Some(1)),
            entry("large", EntryKind::File, Some(8)),
            entry("dir", EntryKind::Dir, None),
        ];
        entries.sort_by(|a, b| options.compare(a, b));
        assert_eq!(
            entries.map(|entry| entry.name),
            ["dir", "large", "small", "unknown"]
        );
    }

    #[test]
    fn prefix_rewrites_are_component_based_and_reveal_detects_hidden_ancestors() {
        let path = |text: &str| text.parse::<RelPath>().unwrap();
        assert_eq!(
            replace_prefix(&path("src/a.rs"), &path("src"), &path("lib")),
            Some(path("lib/a.rs"))
        );
        assert_eq!(
            replace_prefix(&path("src/sub/a.rs"), &path("src"), &path("lib")),
            Some(path("lib/sub/a.rs"))
        );
        assert_eq!(
            replace_prefix(&path("src-old/a.rs"), &path("src"), &path("lib")),
            None
        );
        assert!(hidden_path(&path("src/.config/a")));
        assert!(!hidden_path(&path("src/config.a")));
    }
}
