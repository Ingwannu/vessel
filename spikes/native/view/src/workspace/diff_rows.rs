//! A file's diff as rows for a windowed list: a header per hunk, then its
//! lines with their numbers on each side (agent-orchestrator's
//! `WorkspaceDiffView`, GOAT's and Paseo's unified diff; the page's
//! `data/workspace/diff-rows.ts`, ported with its cases).
//!
//! [Decision Log]
//! - Purpose: the Changes tab draws every changed file's diff one under
//!   another, and a diff may be thousands of lines: never draw a whole
//!   large diff at once.
//! - Constraint: the node pages a file's diff by hunk (5,000 lines a call,
//!   `more_from`), so a file's rows grow as the reader goes; the list is
//!   windowed, which wants a stable key per row.
//! - Choice: one flat list of rows per file, each keyed by the hunk's index
//!   in the file and the line's place in it (stable as pages are added),
//!   the old and new line numbers worked out once, so a drawn row only
//!   reads. The page keys rows by strings with the path in them; here a
//!   key is a small value, and the file it belongs to is the list's.
//! - Cost: a 5,000-line page is 5,000 small rows, made once per page.
//! - A hunk longer than a page is cut by the node: its lines then fall
//!   short of what its header counts, and a `Cut` row after them says so.

use std::sync::Arc;

use vessel_proto::workspace::{DiffContent, FileDiff, Hunk, LineKind};

/// Which row of a file's diff, stable as pages are added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffKey {
    /// The header of the file's `n`th hunk.
    Hunk(u32),
    /// Line `line` of hunk `hunk`.
    Line {
        /// The hunk's index in the file.
        hunk: u32,
        /// The line's index in the hunk.
        line: u32,
    },
    /// The cut after hunk `n`.
    Cut(u32),
}

/// One row of a file's diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffRow {
    /// A hunk's header.
    Hunk {
        /// Which.
        key: DiffKey,
        /// `@@ -12,7 +12,9 @@`, as git writes it.
        header: String,
        /// The function or section git names after it, if any.
        section: Option<String>,
    },
    /// A line.
    Line {
        /// Which.
        key: DiffKey,
        /// What it is.
        line: LineKind,
        /// Its text (shared with the page it came in).
        text: Arc<str>,
        /// Whether the node cut it (over 4 KiB).
        cut: bool,
        /// Its number on the old side, when it is there.
        old: Option<u32>,
        /// Its number on the new side, when it is there.
        new: Option<u32>,
    },
    /// The node sent only the start of this hunk.
    Cut {
        /// Which.
        key: DiffKey,
    },
}

impl DiffRow {
    /// The row's key.
    #[must_use]
    pub const fn key(&self) -> DiffKey {
        match self {
            Self::Hunk { key, .. } | Self::Line { key, .. } | Self::Cut { key } => *key,
        }
    }
}

/// A hunk's header as git writes it.
#[must_use]
pub fn hunk_header(hunk: &Hunk) -> String {
    format!(
        "@@ -{},{} +{},{} @@",
        hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines
    )
}

/// A line's numbers on each side, and the numbers after it.
const fn numbered(kind: LineKind, old: u32, new: u32) -> (Option<u32>, Option<u32>, u32, u32) {
    match kind {
        LineKind::Context => (
            Some(old),
            Some(new),
            old.saturating_add(1),
            new.saturating_add(1),
        ),
        LineKind::Added => (None, Some(new), old, new.saturating_add(1)),
        LineKind::Removed => (Some(old), None, old.saturating_add(1), new),
        LineKind::NoNewline => (None, None, old, new),
    }
}

/// The rows of `hunk`, the `index`th of its file, onto `rows`.
fn hunk_rows(hunk: &Hunk, index: u32, rows: &mut Vec<DiffRow>) {
    rows.push(DiffRow::Hunk {
        key: DiffKey::Hunk(index),
        header: hunk_header(hunk),
        section: hunk.section.clone(),
    });
    let (mut old, mut new) = (hunk.old_start, hunk.new_start);
    let (mut on_old, mut on_new) = (0_u32, 0_u32);
    for (at, line) in hunk.lines.iter().enumerate() {
        if matches!(line.kind, LineKind::Context | LineKind::Removed) {
            on_old = on_old.saturating_add(1);
        }
        if matches!(line.kind, LineKind::Context | LineKind::Added) {
            on_new = on_new.saturating_add(1);
        }
        let (old_number, new_number, next_old, next_new) = numbered(line.kind, old, new);
        old = next_old;
        new = next_new;
        rows.push(DiffRow::Line {
            key: DiffKey::Line {
                hunk: index,
                line: u32::try_from(at).unwrap_or(u32::MAX),
            },
            line: line.kind,
            text: Arc::from(line.text.as_str()),
            cut: line.cut,
            old: old_number,
            new: new_number,
        });
    }
    if on_old < hunk.old_lines || on_new < hunk.new_lines {
        rows.push(DiffRow::Cut {
            key: DiffKey::Cut(index),
        });
    }
}

/// The rows of a page of a file's hunks, the first being hunk number
/// `first` of the file (0 for the first page, the previous page's
/// `more_from` after).
#[must_use]
pub fn diff_rows(hunks: &[Hunk], first: u32) -> Vec<DiffRow> {
    let lines: usize = hunks.iter().map(|hunk| hunk.lines.len()).sum();
    let mut rows = Vec::with_capacity(lines.saturating_add(hunks.len().saturating_mul(2)));
    let mut index = first;
    for hunk in hunks {
        hunk_rows(hunk, index, &mut rows);
        index = index.saturating_add(1);
    }
    rows
}

/// What a file's diff holds, for its header and its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffBody {
    /// Rows read so far; `more` is the next page's first hunk.
    Rows {
        /// The rows (shared: a page adds to a copy of the pointer list).
        rows: Arc<[DiffRow]>,
        /// Where the next page starts, when there is one.
        more: Option<u32>,
    },
    /// A binary file.
    Binary,
    /// A submodule moved between two commits.
    Submodule {
        /// Before.
        from: Option<String>,
        /// After.
        to: Option<String>,
    },
    /// Larger than the node diffs.
    TooLarge {
        /// Its size.
        bytes: u64,
    },
    /// No content change (a mode change, a rename).
    Unchanged,
    /// Content a newer node knows.
    Unknown,
}

/// `page` added to what was read of the file so far (`before`, `None` for
/// the first page).
#[must_use]
pub fn add_page(before: Option<&DiffBody>, page: &FileDiff) -> DiffBody {
    match &page.content {
        DiffContent::Hunks { hunks, .. } => {
            let (kept, first): (&[DiffRow], u32) = match before {
                Some(DiffBody::Rows { rows, more }) => (rows, more.unwrap_or(0)),
                Some(_) | None => (&[], 0),
            };
            let added = diff_rows(hunks, first);
            let mut rows = Vec::with_capacity(kept.len().saturating_add(added.len()));
            rows.extend_from_slice(kept);
            rows.extend(added);
            DiffBody::Rows {
                rows: rows.into(),
                more: page.more_from,
            }
        }
        DiffContent::Binary => DiffBody::Binary,
        DiffContent::Submodule { from, to, .. } => DiffBody::Submodule {
            from: from.as_ref().map(ToString::to_string),
            to: to.as_ref().map(ToString::to_string),
        },
        DiffContent::TooLarge { bytes, .. } => DiffBody::TooLarge { bytes: *bytes },
        DiffContent::Unchanged => DiffBody::Unchanged,
        DiffContent::Unknown => DiffBody::Unknown,
    }
}

/// Lines added and removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    /// Added.
    pub added: u64,
    /// Removed.
    pub removed: u64,
}

/// The lines `rows` add and remove (a page's own count, for a file whose
/// summary had none).
#[must_use]
pub fn count_lines(rows: &[DiffRow]) -> Counts {
    rows.iter()
        .fold(Counts::default(), |counts, row| match row {
            DiffRow::Line {
                line: LineKind::Added,
                ..
            } => Counts {
                added: counts.added.saturating_add(1),
                ..counts
            },
            DiffRow::Line {
                line: LineKind::Removed,
                ..
            } => Counts {
                removed: counts.removed.saturating_add(1),
                ..counts
            },
            DiffRow::Line { .. } | DiffRow::Hunk { .. } | DiffRow::Cut { .. } => counts,
        })
}

#[cfg(test)]
mod tests {
    //! The page's `diff-rows.test.ts`, case for case.

    use std::collections::HashSet;

    use vessel_proto::workspace::{ChangeStatus, DiffLine};

    use super::*;

    fn line(kind: LineKind, text: &str) -> DiffLine {
        DiffLine {
            kind,
            text: text.to_owned(),
            cut: false,
        }
    }

    fn hunk() -> Hunk {
        Hunk {
            old_start: 10,
            old_lines: 3,
            new_start: 10,
            new_lines: 4,
            section: Some("fn main()".to_owned()),
            lines: vec![
                line(LineKind::Context, "let a = 1;"),
                line(LineKind::Removed, "let b = 2;"),
                line(LineKind::Added, "let b = 3;"),
                line(LineKind::Added, "let c = 4;"),
                line(LineKind::Context, "}"),
                line(LineKind::NoNewline, "\\ No newline at end of file"),
            ],
        }
    }

    fn page(hunks: Vec<Hunk>, more_from: Option<u32>) -> FileDiff {
        FileDiff {
            path: "src/main.rs".parse().unwrap(),
            old_path: None,
            status: ChangeStatus::Modified,
            mode: None,
            content: DiffContent::Hunks { hunks },
            more_from,
        }
    }

    #[test]
    fn writes_the_hunk_s_header_as_git_does() {
        assert_eq!(hunk_header(&hunk()), "@@ -10,3 +10,4 @@");
    }

    #[test]
    fn numbers_each_side_as_it_goes() {
        let rows = diff_rows(&[hunk()], 0);
        let numbers: Vec<_> = rows
            .iter()
            .filter_map(|row| match row {
                DiffRow::Line { old, new, .. } => Some((*old, *new)),
                DiffRow::Hunk { .. } | DiffRow::Cut { .. } => None,
            })
            .collect();
        assert_eq!(
            numbers,
            [
                (Some(10), Some(10)),
                (Some(11), None),
                (None, Some(11)),
                (None, Some(12)),
                (Some(12), Some(13)),
                (None, None),
            ]
        );
        assert!(matches!(
            &rows[0],
            DiffRow::Hunk { section: Some(section), .. } if section == "fn main()"
        ));
    }

    #[test]
    fn says_when_the_node_sent_only_the_start_of_a_hunk() {
        assert!(
            !diff_rows(&[hunk()], 0)
                .iter()
                .any(|row| matches!(row, DiffRow::Cut { .. }))
        );
        let long = Hunk {
            old_lines: 9000,
            new_lines: 9001,
            ..hunk()
        };
        assert_eq!(
            diff_rows(&[long], 0).last(),
            Some(&DiffRow::Cut {
                key: DiffKey::Cut(0)
            })
        );
    }

    #[test]
    fn keeps_keys_stable_as_pages_are_added() {
        let first = add_page(None, &page(vec![hunk()], Some(1)));
        let second = add_page(Some(&first), &page(vec![hunk()], None));
        let DiffBody::Rows { rows, more } = second else {
            panic!("rows expected");
        };
        let keys: Vec<DiffKey> = rows.iter().map(DiffRow::key).collect();
        assert_eq!(keys.iter().collect::<HashSet<_>>().len(), keys.len());
        assert!(keys.contains(&DiffKey::Hunk(1)));
        assert_eq!(more, None);
    }

    #[test]
    fn says_what_a_binary_or_oversized_file_is_instead_of_rows() {
        let binary = FileDiff {
            content: DiffContent::Binary,
            ..page(Vec::new(), None)
        };
        assert_eq!(add_page(None, &binary), DiffBody::Binary);
        let large = FileDiff {
            content: DiffContent::TooLarge { bytes: 9 },
            ..page(Vec::new(), None)
        };
        assert_eq!(add_page(None, &large), DiffBody::TooLarge { bytes: 9 });
    }

    #[test]
    fn counts_what_a_page_adds_and_removes() {
        assert_eq!(
            count_lines(&diff_rows(&[hunk()], 0)),
            Counts {
                added: 2,
                removed: 1
            }
        );
    }
}
