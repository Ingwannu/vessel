//! Diff paths only: one ordering authority for the rail and scrolling document.
//! Filesystem explorer state is deliberately not shared with this hierarchy.
//!
//! [Decision Log]
//! - Purpose: Paseo diff-tree/diff-order agree even for root dotfiles and a
//!   file replaced by a same-named directory.
//! - Constraint: input path depth is external; recursive tree building/visiting
//!   can spend stack on it, and absent wire counts are not measured zeroes.
//! - Choice: an arena with iterative build, bottom-up totals and iterative walk.
//! - Effect: compressed rows keep the deepest full path; flattening cannot
//!   change file identity or reorder equal paths. No new dependency is needed.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use vessel_proto::workspace::{FileChange, RelPath};

/// Count of source lines, distinct from files and visible rows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineCount(u64);

impl LineCount {
    /// A protocol count fits without narrowing.
    #[must_use]
    pub fn from_wire(value: u32) -> Self {
        Self(u64::from(value))
    }

    /// Used at formatting and geometry boundaries only.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    fn plus(self, other: Self) -> Self {
        // Input is bounded to 5,000 u32 counts; saturation also keeps malformed
        // locally restored summaries from overflowing a release build.
        Self(self.0.saturating_add(other.0))
    }
}

/// Unknown counts stay unknown through a subtree, including binary files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeTotals {
    /// Added lines, only when every descendant supplied that count.
    pub additions: Option<LineCount>,
    /// Removed lines, independently known or unknown.
    pub deletions: Option<LineCount>,
}

impl Default for ChangeTotals {
    fn default() -> Self {
        Self {
            additions: Some(LineCount::default()),
            deletions: Some(LineCount::default()),
        }
    }
}

impl ChangeTotals {
    fn of(file: &FileChange) -> Self {
        Self {
            additions: file.additions.map(LineCount::from_wire),
            deletions: file.deletions.map(LineCount::from_wire),
        }
    }

    fn plus(self, other: Self) -> Self {
        Self {
            additions: self.additions.zip(other.additions).map(|(a, b)| a.plus(b)),
            deletions: self.deletions.zip(other.deletions).map(|(a, b)| a.plus(b)),
        }
    }
}

/// Zero-based indentation level, independent of pixel width.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TreeDepth(usize);

impl TreeDepth {
    /// Renderer converts this logical level to its named indentation step.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }

    fn child(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Source position retained even when directory-first ordering changes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeIndex(usize);

impl ChangeIndex {
    /// Used only with checked access to the original summary.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// Flat virtual-list rows; folder identity is not its compressed display text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeTreeRow<'a> {
    /// Aggregate stays complete even when this row is folded.
    Folder {
        /// Deepest uncompressed path in the chain.
        path: &'a RelPath,
        /// Compressed names relative to the visible parent.
        label: Arc<str>,
        /// Visible indentation level.
        depth: TreeDepth,
        /// Totals of every descendant, folded or not.
        totals: ChangeTotals,
    },
    /// Original file data is borrowed, never reduced to just name/counts.
    File {
        /// Original summary entry, preserving rename/binary/status detail.
        file: &'a FileChange,
        /// Original position for a body cache or selection.
        index: ChangeIndex,
        /// Visible indentation level.
        depth: TreeDepth,
    },
}

#[derive(Debug)]
struct Directory {
    path: RelPath,
    name: String,
    directories: BTreeMap<String, usize>,
    files: Vec<usize>,
    totals: ChangeTotals,
}

impl Directory {
    fn new(path: RelPath, name: String) -> Self {
        Self {
            path,
            name,
            directories: BTreeMap::new(),
            files: Vec::new(),
            totals: ChangeTotals::default(),
        }
    }
}

/// Arena created once per summary; all surfaces derive file order from it.
#[derive(Debug)]
pub struct ChangeTree<'a> {
    files: &'a [FileChange],
    directories: Vec<Directory>,
}

impl<'a> ChangeTree<'a> {
    /// Validated `RelPath` components cannot escape the virtual root.
    #[must_use]
    pub fn new(files: &'a [FileChange]) -> Self {
        let mut tree = Self {
            files,
            directories: vec![Directory::new(RelPath::root(), String::new())],
        };
        for (index, file) in files.iter().enumerate() {
            tree.insert(index, file);
        }
        tree.aggregate();
        tree
    }

    fn insert(&mut self, index: usize, file: &FileChange) {
        let mut parent = 0;
        let mut ancestors = Vec::new();
        let mut path = file.path.parent();
        while let Some(directory) = path.filter(|path| !path.is_root()) {
            path = directory.parent();
            ancestors.push(directory);
        }
        for path in ancestors.into_iter().rev() {
            let component = path.file_name().unwrap_or_default();
            let existing = self
                .directories
                .get(parent)
                .and_then(|dir| dir.directories.get(component))
                .copied();
            parent = if let Some(existing) = existing {
                existing
            } else {
                let name = component.to_owned();
                let child = self.directories.len();
                self.directories.push(Directory::new(path, name.clone()));
                if let Some(directory) = self.directories.get_mut(parent) {
                    let _ = directory.directories.insert(name, child);
                }
                child
            };
        }
        if let Some(directory) = self.directories.get_mut(parent) {
            directory.files.push(index);
        }
    }

    fn aggregate(&mut self) {
        for index in (0..self.directories.len()).rev() {
            let Some(directory) = self.directories.get(index) else {
                continue;
            };
            let files = directory
                .files
                .iter()
                .filter_map(|file| self.files.get(*file));
            let mut totals = files.fold(ChangeTotals::default(), |sum, file| {
                sum.plus(ChangeTotals::of(file))
            });
            for child in directory
                .directories
                .values()
                .filter_map(|child| self.directories.get(*child))
            {
                totals = totals.plus(child.totals);
            }
            if let Some(directory) = self.directories.get_mut(index) {
                directory.totals = totals;
            }
        }
    }

    /// Expand only visible descendants, using full compressed-row identities.
    #[must_use]
    pub fn rows(&self, collapsed: &BTreeSet<RelPath>) -> Vec<ChangeTreeRow<'_>> {
        let mut rows = Vec::new();
        let mut pending = self.children(0, TreeDepth::default());
        pending.reverse();
        while let Some((item, depth)) = pending.pop() {
            match item {
                Item::File(index) => {
                    if let Some(file) = self.files.get(index) {
                        rows.push(ChangeTreeRow::File {
                            file,
                            index: ChangeIndex(index),
                            depth,
                        });
                    }
                }
                Item::Directory(index) => {
                    let Some((deepest, label)) = self.compressed(index) else {
                        continue;
                    };
                    let Some(directory) = self.directories.get(deepest) else {
                        continue;
                    };
                    rows.push(ChangeTreeRow::Folder {
                        path: &directory.path,
                        label: Arc::from(label),
                        depth,
                        totals: directory.totals,
                    });
                    if !collapsed.contains(&directory.path) {
                        let mut children = self.children(deepest, depth.child());
                        children.reverse();
                        pending.extend(children);
                    }
                }
            }
        }
        rows
    }

    /// The document ignores rail folding but shares its directory-first order.
    #[must_use]
    pub fn ordered_files(&self) -> Vec<&FileChange> {
        self.rows(&BTreeSet::new())
            .into_iter()
            .filter_map(|row| match row {
                ChangeTreeRow::File { file, .. } => Some(file),
                ChangeTreeRow::Folder { .. } => None,
            })
            .collect()
    }

    fn compressed(&self, mut index: usize) -> Option<(usize, String)> {
        let mut label = self.directories.get(index)?.name.clone();
        loop {
            let directory = self.directories.get(index)?;
            if !directory.files.is_empty() || directory.directories.len() != 1 {
                break;
            }
            let child = *directory.directories.values().next()?;
            let child_dir = self.directories.get(child)?;
            label.push('/');
            label.push_str(&child_dir.name);
            index = child;
        }
        Some((index, label))
    }

    fn children(&self, index: usize, depth: TreeDepth) -> Vec<(Item, TreeDepth)> {
        let Some(directory) = self.directories.get(index) else {
            return Vec::new();
        };
        let mut directories: Vec<_> = directory.directories.iter().collect();
        directories.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
        let mut files: Vec<_> = directory
            .files
            .iter()
            .filter_map(|index| self.files.get(*index).map(|file| (*index, file)))
            .collect();
        // Stable sort intentionally preserves input order when paths are equal.
        files.sort_by(|(_, a), (_, b)| {
            a.path
                .as_str()
                .encode_utf16()
                .cmp(b.path.as_str().encode_utf16())
        });
        directories
            .into_iter()
            .map(|(_, index)| (Item::Directory(*index), depth))
            .chain(
                files
                    .into_iter()
                    .map(|(index, _)| (Item::File(index), depth)),
            )
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
enum Item {
    Directory(usize),
    File(usize),
}
