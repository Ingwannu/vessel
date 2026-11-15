//! Diff paths only: one ordering authority for the rail and scrolling document.
//! Filesystem explorer state is deliberately not shared with this hierarchy.
//!
//! [Decision Log]
//! - Purpose: Paseo diff-tree/diff-order agree even for root dotfiles and a
//!   file replaced by a same-named directory.
//! - Constraint: input path depth is external; recursive tree building/visiting
//!   can spend stack on it, and absent wire counts are not measured zeroes.
//! - Choice: build an already-compressed arena by grouping file prefixes,
//!   then aggregate bottom-up and walk iteratively. Allocating every prefix of
//!   every deep path can amplify a small valid reply into gigabytes of state.
//! - Effect: compressed rows keep the deepest full path; flattening cannot
//!   change file identity or reorder equal paths. No new dependency is needed.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use vessel_proto::workspace::{FileChange, InvalidRelPath, RelPath};

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
    name: Arc<str>,
    directories: BTreeMap<String, usize>,
    files: Vec<usize>,
    totals: ChangeTotals,
}

impl Directory {
    fn new(path: RelPath, name: &str) -> Self {
        Self {
            path,
            name: Arc::from(name),
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
    /// Build only branching/file-bearing directories, with no intermediate
    /// chain nodes. At most one folder per file is created at each branching
    /// boundary, rather than one allocation for every path component.
    ///
    /// # Errors
    /// A derived directory cannot be represented. Keep this typed rather than
    /// dropping a file if relative-path validation changes in the future.
    pub fn new(files: &'a [FileChange]) -> Result<Self, TreeError> {
        let mut tree = Self {
            files,
            directories: vec![Directory::new(RelPath::root(), "")],
        };
        tree.build()?;
        tree.aggregate();
        Ok(tree)
    }

    fn build(&mut self) -> Result<(), TreeError> {
        let files = self.files;
        // Internal byte order makes each directory group a contiguous range.
        // Its first/last parent determine the common directory in one pass;
        // visible order still comes exclusively from children()'s UTF-16 rule.
        let mut indices: Vec<_> = (0..files.len()).collect();
        indices.sort_by_key(|index| files.get(*index).map(|file| file.path.as_str()));
        let mut pending = vec![(0, indices)];
        while let Some((parent, indices)) = pending.pop() {
            let directory = self.directories.get(parent).ok_or(TreeError::Identity)?;
            let prefix = if directory.path.is_root() {
                0
            } else {
                directory
                    .path
                    .as_str()
                    .len()
                    .checked_add(1)
                    .ok_or(TreeError::Identity)?
            };
            let mut groups = BTreeMap::<&str, Vec<usize>>::new();
            for index in indices {
                let file = files.get(index).ok_or(TreeError::Identity)?;
                let remainder = file
                    .path
                    .as_str()
                    .get(prefix..)
                    .ok_or(TreeError::Identity)?;
                if let Some((first, _)) = remainder.split_once('/') {
                    groups.entry(first).or_default().push(index);
                } else {
                    self.directories
                        .get_mut(parent)
                        .ok_or(TreeError::Identity)?
                        .files
                        .push(index);
                }
            }
            for (first, group) in groups {
                let path = common_directory(files, &group)?;
                let name = path.get(prefix..).ok_or(TreeError::Identity)?;
                let directory = Directory::new(path.parse()?, name);
                let child = self.directories.len();
                self.directories.push(directory);
                let _ = self
                    .directories
                    .get_mut(parent)
                    .ok_or(TreeError::Identity)?
                    .directories
                    .insert(first.to_owned(), child);
                pending.push((child, group));
            }
        }
        Ok(())
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
                    let Some(directory) = self.directories.get(index) else {
                        continue;
                    };
                    rows.push(ChangeTreeRow::Folder {
                        path: &directory.path,
                        label: Arc::clone(&directory.name),
                        depth,
                        totals: directory.totals,
                    });
                    if !collapsed.contains(&directory.path) {
                        let mut children = self.children(index, depth.child());
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

/// Tree construction never drops a node-sent file on an internal mismatch.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TreeError {
    /// Preserve the original validated-path conversion failure.
    #[error("a changed-file directory cannot be represented")]
    Path(#[from] InvalidRelPath),
    /// A source position or prefix did not belong to its directory.
    #[error("changed-file directory identities are inconsistent")]
    Identity,
}

fn common_directory<'a>(files: &'a [FileChange], indices: &[usize]) -> Result<&'a str, TreeError> {
    let parent = |index: &usize| {
        files
            .get(*index)
            .and_then(|file| {
                file.path
                    .as_str()
                    .rsplit_once('/')
                    .map(|(parent, _)| parent)
            })
            .ok_or(TreeError::Identity)
    };
    let first = parent(indices.first().ok_or(TreeError::Identity)?)?;
    let last = parent(indices.last().ok_or(TreeError::Identity)?)?;
    let shared = common_prefix(first, last)?;
    if shared.is_empty() {
        return Err(TreeError::Identity);
    }
    Ok(shared)
}

fn common_prefix<'a>(a: &'a str, b: &str) -> Result<&'a str, TreeError> {
    let shared = a.bytes().zip(b.bytes()).take_while(|(a, b)| a == b).count();
    let end = if (shared == a.len() && b.as_bytes().get(shared).is_none_or(|byte| *byte == b'/'))
        || (shared == b.len() && a.as_bytes().get(shared) == Some(&b'/'))
    {
        shared
    } else {
        // A mismatch can split a UTF-8 scalar, but an ASCII slash is always a
        // scalar boundary. Never slice text at the raw mismatching byte.
        a.as_bytes()
            .iter()
            .take(shared)
            .rposition(|byte| *byte == b'/')
            .unwrap_or(0)
    };
    a.get(..end).ok_or(TreeError::Identity)
}
