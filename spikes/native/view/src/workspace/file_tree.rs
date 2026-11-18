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

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use vessel_proto::workspace::{DirEntry, DirListing, EntryKind, RelPath};

use crate::prefs::ExplorerSort;

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

/// When an entry last changed and its size (`file.inspect`), for the
/// Modified and Size orders; `None` where the node did not say.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EntryFacts {
    /// Milliseconds since the epoch.
    pub modified: Option<u64>,
    /// Bytes.
    pub size: Option<u64>,
}

/// How the tree orders a folder's entries, and the facts it orders by,
/// by path (none needed for Name).
pub struct Sorting<'facts, S> {
    /// The order.
    pub by: ExplorerSort,
    /// The facts.
    pub facts: &'facts HashMap<RelPath, EntryFacts, S>,
}

// By hand: a derive would ask the hasher to be `Copy` too.
impl<S> Clone for Sorting<'_, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for Sorting<'_, S> {}

// By hand for the same reason: the map is `Debug` whatever its hasher.
impl<S> std::fmt::Debug for Sorting<'_, S> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Sorting")
            .field("by", &self.by)
            .field("facts", self.facts)
            .finish()
    }
}

/// Names as people read them (the page's `localeCompare`): letters before
/// their case, then lower case first.
fn by_name(one: &str, two: &str) -> Ordering {
    one.to_lowercase()
        .cmp(&two.to_lowercase())
        .then_with(|| two.cmp(one))
}

/// Indexes of `listing`'s entries in Paseo's explorer order (tree.ts
/// `sortExplorerEntries`): folders first, then by name, by last change
/// (newest first) or by size (largest first); an entry with no fact goes
/// last, and ties keep the node's order.
fn ordered<S: std::hash::BuildHasher>(
    listing: &DirListing,
    dir: &RelPath,
    sorting: Sorting<'_, S>,
) -> Vec<usize> {
    let fact = |entry: &DirEntry| -> Option<u64> {
        let facts = sorting.facts.get(&dir.child(&entry.name).ok()?)?;
        match sorting.by {
            ExplorerSort::Name => None,
            ExplorerSort::Modified => facts.modified,
            ExplorerSort::Size => facts.size,
        }
    };
    let mut order: Vec<usize> = (0..listing.entries.len()).collect();
    order.sort_by(|&one, &two| {
        let (Some(a), Some(b)) = (listing.entries.get(one), listing.entries.get(two)) else {
            return Ordering::Equal;
        };
        let folder = |entry: &DirEntry| entry.kind == EntryKind::Dir;
        folder(b).cmp(&folder(a)).then_with(|| match sorting.by {
            ExplorerSort::Name => by_name(&a.name, &b.name),
            // Larger first; `None` (no fact) is below every `Some`.
            ExplorerSort::Modified | ExplorerSort::Size => fact(b).cmp(&fact(a)),
        })
    });
    order
}

/// The rows of the tree from its root.
#[must_use]
pub fn tree_rows<S: std::hash::BuildHasher>(
    listings: &HashMap<RelPath, Listing, S>,
    open: &HashSet<RelPath, S>,
    sorting: Sorting<'_, S>,
) -> Vec<TreeRow> {
    let mut rows = Vec::new();
    rows_under(listings, open, sorting, &RelPath::root(), 0, &mut rows);
    rows
}

fn rows_under<S: std::hash::BuildHasher>(
    listings: &HashMap<RelPath, Listing, S>,
    open: &HashSet<RelPath, S>,
    sorting: Sorting<'_, S>,
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
    for index in ordered(listing, dir, sorting) {
        let Some(entry) = listing.entries.get(index) else {
            continue;
        };
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
            rows_under(
                listings,
                open,
                sorting,
                &path,
                depth.saturating_add(1),
                rows,
            );
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

    static NO_FACTS: std::sync::LazyLock<HashMap<RelPath, EntryFacts>> =
        std::sync::LazyLock::new(HashMap::new);

    fn by_name_only() -> Sorting<'static, std::collections::hash_map::RandomState> {
        Sorting {
            by: ExplorerSort::Name,
            facts: &NO_FACTS,
        }
    }

    /// The names of the root's entry rows, in order.
    fn names(
        listing: &DirListing,
        sorting: Sorting<'_, std::collections::hash_map::RandomState>,
    ) -> Vec<String> {
        let listings = HashMap::from([(RelPath::root(), Listing::Listed(listing.clone()))]);
        tree_rows(&listings, &HashSet::new(), sorting)
            .iter()
            .filter_map(|row| {
                if let TreeRow::Entry { index, .. } = row {
                    listing.entries.get(*index).map(|entry| entry.name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// The page's file-tree.test.ts "Paseo's explorer orders".
    #[test]
    fn orders_folders_first_then_by_name_change_or_size() {
        let listing = DirListing {
            entries: vec![
                entry("b.txt", EntryKind::File),
                entry("src", EntryKind::Dir),
                entry("a.txt", EntryKind::File),
                entry("C.txt", EntryKind::File),
                entry("docs", EntryKind::Dir),
            ],
            more: false,
        };
        let fact = |modified, size| EntryFacts {
            modified: Some(modified),
            size,
        };
        let facts = HashMap::from([
            (path("a.txt"), fact(100, Some(3000))),
            (path("b.txt"), fact(300, Some(10))),
            (path("C.txt"), fact(200, Some(500))),
            (path("src"), fact(50, None)),
            (path("docs"), fact(400, None)),
        ]);
        let by = |by| Sorting { by, facts: &facts };
        assert_eq!(
            names(&listing, by(ExplorerSort::Name)),
            ["docs", "src", "a.txt", "b.txt", "C.txt"]
        );
        assert_eq!(
            names(&listing, by(ExplorerSort::Modified)),
            ["docs", "src", "b.txt", "C.txt", "a.txt"]
        );
        assert_eq!(
            names(&listing, by(ExplorerSort::Size)),
            ["src", "docs", "a.txt", "C.txt", "b.txt"]
        );
        // An entry the node gave no time for goes last; ties keep the node's order.
        let some = HashMap::from([(path("a.txt"), fact(1, Some(1)))]);
        let sorted = names(
            &listing,
            Sorting {
                by: ExplorerSort::Modified,
                facts: &some,
            },
        );
        assert_eq!(
            sorted.get(2..),
            Some(&["a.txt".to_owned(), "b.txt".to_owned(), "C.txt".to_owned()][..])
        );
        assert_eq!(ExplorerSort::Name.next(), ExplorerSort::Modified);
        assert_eq!(ExplorerSort::Size.next(), ExplorerSort::Name);
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
        let rows = tree_rows(&listings, &HashSet::from([path("src")]), by_name_only());
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
        let rows = tree_rows(&listings, &HashSet::new(), by_name_only());
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
