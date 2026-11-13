//! The Changes tab's model: which comparison it shows (agent-orchestrator's
//! changes picker), each file's header (its status letter and counts), and
//! every changed file's diff as one list of rows for one windowed list.
//! The page's `data/workspace/changes.ts`, ported with its cases.
//!
//! [Decision Log]
//! - Purpose: agent-orchestrator's review pane draws every changed file one
//!   under another, each a header over its diff, collapsible; a diff is
//!   paged, asked for when the reader gets there, never drawn whole.
//! - Constraint: the node answers the summary (at most 5,000 files) and a
//!   file's diff (5,000 lines a page) separately; diffs are asked for only
//!   when their place in the list is drawn.
//! - Choice: one flat list of rows, a file's header then, when it is open,
//!   its diff's rows or a row saying what the diff is (loading, binary, too
//!   large) or that there is more of it. A row names its file and its diff
//!   row by index, so the list is small values the window reads through;
//!   the page's rows carry string keys and object references instead.
//! - Cost: the list is rebuilt when a page arrives (the rows' count, not
//!   their text).

use std::collections::{HashMap, HashSet};

use vessel_proto::workspace::{ChangeStatus, DiffScope, FileChange, RelPath};

use super::diff_rows::{Counts, DiffBody, DiffKey};

/// The comparisons the picker offers, in its order (the latest commits
/// follow them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// Everything since the base branch.
    Base,
    /// Not committed.
    Uncommitted,
    /// Staged.
    Staged,
    /// Not staged.
    Unstaged,
}

impl ScopeKind {
    /// The picker's order.
    pub const ALL: [Self; 4] = [Self::Base, Self::Uncommitted, Self::Staged, Self::Unstaged];

    /// The comparison asked of the node.
    #[must_use]
    pub const fn scope(self) -> DiffScope {
        match self {
            Self::Base => DiffScope::Base { base: None },
            Self::Uncommitted => DiffScope::Uncommitted,
            Self::Staged => DiffScope::Staged,
            Self::Unstaged => DiffScope::Unstaged,
        }
    }
}

/// The tone a file's status is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// New.
    Added,
    /// Gone or conflicted.
    Removed,
    /// Edited.
    Modified,
    /// Renamed or copied.
    Renamed,
    /// A status a newer node knows.
    Other,
}

/// A file's status as its header's letter, and the tone it is drawn in.
#[must_use]
pub const fn status_letter(status: ChangeStatus) -> (char, Tone) {
    match status {
        ChangeStatus::Added => ('A', Tone::Added),
        ChangeStatus::Deleted => ('D', Tone::Removed),
        ChangeStatus::Modified => ('M', Tone::Modified),
        ChangeStatus::Renamed => ('R', Tone::Renamed),
        ChangeStatus::Copied => ('C', Tone::Renamed),
        ChangeStatus::TypeChanged => ('T', Tone::Modified),
        ChangeStatus::Conflicted => ('U', Tone::Removed),
        ChangeStatus::Unknown => ('?', Tone::Other),
    }
}

/// A path as its folder (with its slash) and its name, drawn apart.
#[must_use]
pub fn split_path(path: &str) -> (&str, &str) {
    path.rfind('/')
        .and_then(|at| at.checked_add(1))
        .map_or(("", path), |cut| path.split_at(cut))
}

/// What one file's diff is known to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    /// Not asked for.
    Unread,
    /// Asked for; what was read before, when anything was.
    Loading(Option<DiffBody>),
    /// Read.
    Read(DiffBody),
    /// The read failed; the words to show.
    Failed(String),
}

/// What a row in place of (or after) a file's diff says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileNote {
    /// The diff is being read.
    Loading,
    /// The file's diff has another page.
    More,
    /// A binary file.
    Binary,
    /// Larger than the node diffs.
    TooLarge {
        /// Its size.
        bytes: u64,
    },
    /// A submodule moved.
    Submodule {
        /// Before.
        from: Option<String>,
        /// After.
        to: Option<String>,
    },
    /// No content change.
    Unchanged,
    /// Content a newer node knows.
    Unknown,
    /// The read failed.
    Failed(String),
}

/// One row of the Changes list, naming what it draws by index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeRow {
    /// File `file`'s header.
    File {
        /// Its index in the summary.
        file: usize,
        /// Whether its diff is shown.
        open: bool,
    },
    /// Row `row` of file `file`'s diff (its body's rows).
    Diff {
        /// The file's index in the summary.
        file: usize,
        /// The row's index in the file's rows.
        row: usize,
        /// The row's key.
        key: DiffKey,
    },
    /// A note under file `file`.
    Note {
        /// The file's index in the summary.
        file: usize,
        /// What it says.
        note: FileNote,
    },
}

fn body_rows(file: usize, body: &DiffBody, rows: &mut Vec<ChangeRow>) {
    let note = |note: FileNote| ChangeRow::Note { file, note };
    match body {
        DiffBody::Rows { rows: diff, more } => {
            rows.extend(diff.iter().enumerate().map(|(row, line)| ChangeRow::Diff {
                file,
                row,
                key: line.key(),
            }));
            if more.is_some() {
                rows.push(note(FileNote::More));
            }
        }
        DiffBody::Binary => rows.push(note(FileNote::Binary)),
        DiffBody::Submodule { from, to } => rows.push(note(FileNote::Submodule {
            from: from.clone(),
            to: to.clone(),
        })),
        DiffBody::TooLarge { bytes } => rows.push(note(FileNote::TooLarge { bytes: *bytes })),
        DiffBody::Unchanged => rows.push(note(FileNote::Unchanged)),
        DiffBody::Unknown => rows.push(note(FileNote::Unknown)),
    }
}

/// Every row of the list: each file's header and, when open, its diff.
#[must_use]
pub fn change_rows<S: std::hash::BuildHasher>(
    files: &[FileChange],
    closed: &HashSet<RelPath, S>,
    states: &HashMap<RelPath, FileState, S>,
) -> Vec<ChangeRow> {
    let mut rows = Vec::with_capacity(files.len().saturating_mul(2));
    for (file, change) in files.iter().enumerate() {
        let open = !closed.contains(&change.path);
        rows.push(ChangeRow::File { file, open });
        if !open {
            continue;
        }
        match states.get(&change.path) {
            None | Some(FileState::Unread | FileState::Loading(None)) => {
                rows.push(ChangeRow::Note {
                    file,
                    note: FileNote::Loading,
                });
            }
            Some(FileState::Loading(Some(body)) | FileState::Read(body)) => {
                body_rows(file, body, &mut rows);
            }
            Some(FileState::Failed(message)) => rows.push(ChangeRow::Note {
                file,
                note: FileNote::Failed(message.clone()),
            }),
        }
    }
    rows
}

/// Added and removed lines over every file the summary counted.
#[must_use]
pub fn totals(files: &[FileChange]) -> Counts {
    files.iter().fold(Counts::default(), |counts, file| Counts {
        added: counts
            .added
            .saturating_add(u64::from(file.additions.unwrap_or(0))),
        removed: counts
            .removed
            .saturating_add(u64::from(file.deletions.unwrap_or(0))),
    })
}

/// Whether a file's diff read before still stands for `change`: the same
/// status and counts (a refresh keeps it, else it is read again).
#[must_use]
pub fn same_change(before: &FileChange, change: &FileChange) -> bool {
    before.status == change.status
        && before.additions == change.additions
        && before.deletions == change.deletions
        && before.old_path == change.old_path
}

#[cfg(test)]
mod tests {
    //! The page's `panel-model.test.ts`, "the Changes list".

    use vessel_proto::workspace::{DiffLine, Hunk, LineKind};

    use super::super::diff_rows::diff_rows;
    use super::*;

    fn change(path: &str) -> FileChange {
        FileChange {
            path: path.parse().unwrap(),
            old_path: None,
            status: ChangeStatus::Modified,
            additions: Some(2),
            deletions: Some(1),
            binary: false,
        }
    }

    #[test]
    fn letters_a_status_and_splits_a_path_for_its_header() {
        assert_eq!(status_letter(ChangeStatus::Added), ('A', Tone::Added));
        assert_eq!(split_path("src/data/a.ts"), ("src/data/", "a.ts"));
        assert_eq!(split_path("README.md"), ("", "README.md"));
    }

    #[test]
    fn puts_each_file_s_diff_under_its_header_a_loading_row_until_read() {
        let one = change("src/app.ts");
        let other = change("b.ts");
        let rows = diff_rows(
            &[Hunk {
                old_start: 0,
                old_lines: 0,
                new_start: 1,
                new_lines: 1,
                section: None,
                lines: vec![DiffLine {
                    kind: LineKind::Added,
                    text: "x".to_owned(),
                    cut: false,
                }],
            }],
            0,
        );
        let closed = HashSet::from([other.path.clone()]);
        let states = HashMap::from([(
            one.path.clone(),
            FileState::Read(DiffBody::Rows {
                rows: rows.into(),
                more: Some(1),
            }),
        )]);
        let listed = change_rows(&[one.clone(), other], &closed, &states);
        let kinds: Vec<&str> = listed
            .iter()
            .map(|row| match row {
                ChangeRow::File { .. } => "file",
                ChangeRow::Diff {
                    key: DiffKey::Hunk(_),
                    ..
                } => "hunk",
                ChangeRow::Diff { .. } => "line",
                ChangeRow::Note { .. } => "note",
            })
            .collect();
        assert_eq!(kinds, ["file", "hunk", "line", "note", "file"]);
        assert!(matches!(
            &listed[3],
            ChangeRow::Note {
                note: FileNote::More,
                ..
            }
        ));
        let unread = change_rows(&[one], &HashSet::new(), &HashMap::new());
        assert!(matches!(
            &unread[1],
            ChangeRow::Note {
                note: FileNote::Loading,
                ..
            }
        ));
    }

    #[test]
    fn totals_the_counts_and_keeps_a_read_diff_only_while_it_is_the_same() {
        let uncounted = FileChange {
            additions: None,
            ..change("src/app.ts")
        };
        assert_eq!(
            totals(&[change("src/app.ts"), uncounted]),
            Counts {
                added: 2,
                removed: 2
            }
        );
        assert!(same_change(&change("a"), &change("a")));
        let more = FileChange {
            additions: Some(3),
            ..change("a")
        };
        assert!(!same_change(&change("a"), &more));
    }
}
