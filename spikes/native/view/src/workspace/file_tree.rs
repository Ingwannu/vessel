//! The Files tab's tree as rows for a windowed list: the folders opened so
//! far, each listed once by the node (`file.list`, folders first, at most
//! 5,000 entries a folder), flattened in order with their depth. The
//! page's `data/workspace/file-tree.ts`, ported with its cases.
//!
//! [Decision Log]
//! - Purpose: agent-orchestrator's file tree, which lists a folder when it
//!   is opened, not the whole repository up front.
//! - Constraint: a repository may hold hundreds of thousands of files; a
//!   folder's listing is bounded and says `more`; a window of rows is
//!   drawn.
//! - Choice: the listings by folder path, the open folders as a set, and a
//!   pure flattening into rows; a listing that has not arrived yet is one
//!   "loading" row under its folder, which the drawn window asks for. An
//!   entry row names its folder's listing and its index in it, so a row is
//!   small and the listing is not copied.
//! - Cost: a folder closed and opened again is not listed again until the
//!   tab refreshes; a file an agent adds meanwhile shows on the refresh.

use std::collections::{HashMap, HashSet};

use vessel_proto::workspace::{DirListing, EntryKind, RelPath};

/// What is known of one folder's entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listing {
    /// Asked for.
    Loading,
    /// Listed.
    Listed(DirListing),
    /// The listing failed; the words to show.
    Failed(String),
}

/// One row of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeRow {
    /// An entry: entry `index` of folder `dir`'s listing.
    Entry {
        /// The folder it is in.
        dir: RelPath,
        /// Its index in that folder's listing.
        index: usize,
        /// Its own path.
        path: RelPath,
        /// How deep.
        depth: usize,
        /// Whether it is a folder shown open.
        open: bool,
    },
    /// Folder `dir` is being listed.
    Loading {
        /// Which.
        dir: RelPath,
        /// How deep its entries are.
        depth: usize,
    },
    /// Its listing failed.
    Failed {
        /// Which.
        dir: RelPath,
        /// Why.
        message: String,
        /// How deep.
        depth: usize,
    },
    /// The folder has more entries than were listed.
    More {
        /// Which.
        dir: RelPath,
        /// How deep.
        depth: usize,
    },
    /// The folder is empty.
    Empty {
        /// Which.
        dir: RelPath,
        /// How deep.
        depth: usize,
    },
}

impl TreeRow {
    /// How deep the row is drawn.
    #[must_use]
    pub const fn depth(&self) -> usize {
        match self {
            Self::Entry { depth, .. }
            | Self::Loading { depth, .. }
            | Self::Failed { depth, .. }
            | Self::More { depth, .. }
            | Self::Empty { depth, .. } => *depth,
        }
    }
}

/// The rows of the tree from its root.
#[must_use]
pub fn tree_rows<S: std::hash::BuildHasher>(
    listings: &HashMap<RelPath, Listing, S>,
    open: &HashSet<RelPath, S>,
) -> Vec<TreeRow> {
    let mut rows = Vec::new();
    rows_under(listings, open, &RelPath::root(), 0, &mut rows);
    rows
}

fn rows_under<S: std::hash::BuildHasher>(
    listings: &HashMap<RelPath, Listing, S>,
    open: &HashSet<RelPath, S>,
    dir: &RelPath,
    depth: usize,
    rows: &mut Vec<TreeRow>,
) {
    let listing = match listings.get(dir) {
        None | Some(Listing::Loading) => {
            rows.push(TreeRow::Loading {
                dir: dir.clone(),
                depth,
            });
            return;
        }
        Some(Listing::Failed(message)) => {
            rows.push(TreeRow::Failed {
                dir: dir.clone(),
                message: message.clone(),
                depth,
            });
            return;
        }
        Some(Listing::Listed(listing)) => listing,
    };
    if listing.entries.is_empty() {
        rows.push(TreeRow::Empty {
            dir: dir.clone(),
            depth,
        });
    }
    for (index, entry) in listing.entries.iter().enumerate() {
        // A name the wire's path refuses (the node never lists one) has no
        // row: it could not be opened or read.
        let Ok(path) = dir.child(&entry.name) else {
            continue;
        };
        let is_open = entry.kind == EntryKind::Dir && open.contains(&path);
        rows.push(TreeRow::Entry {
            dir: dir.clone(),
            index,
            path: path.clone(),
            depth,
            open: is_open,
        });
        if is_open {
            rows_under(listings, open, &path, depth.saturating_add(1), rows);
        }
    }
    if listing.more {
        rows.push(TreeRow::More {
            dir: dir.clone(),
            depth,
        });
    }
}

/// The folders on the way to `path`, outermost first (to open them).
#[must_use]
pub fn folders_of(path: &RelPath) -> Vec<RelPath> {
    let mut folders = Vec::new();
    let mut at = path.parent();
    while let Some(folder) = at {
        if folder.is_root() {
            break;
        }
        at = folder.parent();
        folders.push(folder);
    }
    folders.reverse();
    folders
}

#[cfg(test)]
mod tests {
    //! The page's `panel-model.test.ts`, "the file tree".

    use vessel_proto::workspace::DirEntry;

    use super::*;

    fn entry(name: &str, kind: EntryKind) -> DirEntry {
        DirEntry {
            name: name.to_owned(),
            kind,
            size: None,
            ignored: false,
        }
    }

    fn path(text: &str) -> RelPath {
        text.parse().unwrap()
    }

    #[test]
    fn flattens_the_open_folders_in_order_with_their_depth() {
        let listings = HashMap::from([
            (
                RelPath::root(),
                Listing::Listed(DirListing {
                    entries: vec![
                        entry("src", EntryKind::Dir),
                        entry("README.md", EntryKind::File),
                    ],
                    more: false,
                }),
            ),
            (path("src"), Listing::Loading),
        ]);
        let rows = tree_rows(&listings, &HashSet::from([path("src")]));
        let shape: Vec<(&str, usize)> = rows
            .iter()
            .map(|row| {
                let kind = match row {
                    TreeRow::Entry { .. } => "entry",
                    TreeRow::Loading { .. } => "loading",
                    TreeRow::Failed { .. } => "failed",
                    TreeRow::More { .. } => "more",
                    TreeRow::Empty { .. } => "empty",
                };
                (kind, row.depth())
            })
            .collect();
        assert_eq!(shape, [("entry", 0), ("loading", 1), ("entry", 0)]);
    }

    #[test]
    fn says_when_a_folder_has_more_than_was_listed_or_nothing() {
        let listings = HashMap::from([(
            RelPath::root(),
            Listing::Listed(DirListing {
                entries: Vec::new(),
                more: true,
            }),
        )]);
        let rows = tree_rows(&listings, &HashSet::new());
        assert!(matches!(
            rows.as_slice(),
            [TreeRow::Empty { .. }, TreeRow::More { .. }]
        ));
    }

    #[test]
    fn knows_a_path_s_folders() {
        assert_eq!(folders_of(&path("a/b/c.ts")), [path("a"), path("a/b")]);
        assert_eq!(path("a/b/c.ts").parent(), Some(path("a/b")));
        assert_eq!(path("c.ts").parent(), Some(RelPath::root()));
    }
}
