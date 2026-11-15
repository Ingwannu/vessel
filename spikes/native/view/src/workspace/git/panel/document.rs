//! Unified/split cell packing after Paseo's `utils/diff-layout.ts`.
//!
//! [Decision Log]
//! - Existing asset: `workspace::diff_rows` already numbers Vessel's typed
//!   hunks and retains truncation/newline markers. It remains the authority.
//! - Choice: retain its shared source rows and pack references by index. A
//!   replacement run pairs by order, context occupies both sides, and every
//!   hunk flushes pending cells. No second text/numbering engine is introduced.
//! - Why: split/unified toggles must preserve source identity for selection,
//!   scroll anchors and later review comments, independently of visual rows.
//! - Difference: Vessel's hunks omit header lines, so source line indices count
//!   actual hunk lines. The canonical review key stays path/side/source number.
//!   Explicit newline/truncation markers are retained as non-reviewable notices
//!   rather than disappearing in split mode as unsupported reference kinds.

mod buffers;
pub use buffers::{BufferColumn, BufferColumns, BufferError, BufferKind, BufferLine};

use std::num::NonZeroU32;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use vessel_proto::workspace::{LineKind, RelPath};

use super::requests::FileStamp;
use super::{FileTicket, Layout};
use crate::workspace::diff_rows::{DiffKey, DiffRow};

/// The old/new side survives split-to-unified changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewSide {
    /// Before this comparison.
    Old,
    /// After this comparison.
    New,
}

/// Source line numbers are one-based; zero cannot become a review target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceLine(NonZeroU32);

impl SourceLine {
    /// Refuse a non-source zero without guessing another line.
    #[must_use]
    pub const fn new(number: u32) -> Option<Self> {
        match NonZeroU32::new(number) {
            Some(number) => Some(Self(number)),
            None => None,
        }
    }

    /// Convert only at the wire/label boundary.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

/// One canonical structured identity, with no path delimiter ambiguities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReviewKey {
    /// Validated relative file, including punctuation or spaces.
    pub path: RelPath,
    /// Context has two distinct keys in split mode.
    pub side: ReviewSide,
    /// Real source line, never a wrapped fragment's visual number.
    pub line: SourceLine,
}

/// Index into the retained source rows, distinct from the displayed row number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRow(usize);

/// A cell retains its source row even when its partner side is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffCell {
    source: SourceRow,
    stamp: FileStamp,
    side: ReviewSide,
    number: Option<SourceLine>,
}

impl DiffCell {
    /// Stable source row, used for text/tokens rather than a display index.
    #[must_use]
    pub const fn source(self) -> SourceRow {
        self.source
    }

    /// The old/new source side; context has a cell for each in split mode.
    #[must_use]
    pub const fn side(self) -> ReviewSide {
        self.side
    }

    /// Reviewable source number, absent for a malformed zero.
    #[must_use]
    pub const fn number(self) -> Option<SourceLine> {
        self.number
    }
}

/// A display row references the existing source model rather than copying text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayRow {
    /// Hunk header spans both columns and has no review target.
    Header(SourceRow),
    /// One source line in unified layout.
    Unified(DiffCell),
    /// Indexed replacement pairs; absence is a blank cell, not an empty line.
    Split {
        /// Old-side removal or context.
        old: Option<DiffCell>,
        /// New-side addition or context.
        new: Option<DiffCell>,
    },
    /// Node truncation or missing-newline marker, without a review target.
    Notice(SourceRow),
}

/// One file's display packing; text and source keys stay shared with its pages.
#[derive(Debug, Clone)]
pub struct DiffDocument {
    path: RelPath,
    layout: Layout,
    source: Arc<[DiffRow]>,
    stamp: FileStamp,
    rows: Vec<DisplayRow>,
}

impl DiffDocument {
    /// Build once per source/layout change, not once per visible-row paint.
    #[must_use]
    pub fn new(ticket: &FileTicket, source: Arc<[DiffRow]>, layout: Layout) -> Self {
        let stamp = ticket.stamp();
        let rows = match layout {
            Layout::Unified => unified(&source, stamp),
            Layout::Split => split(&source, stamp),
        };
        Self {
            path: ticket.path().clone(),
            layout,
            source,
            stamp,
            rows,
        }
    }

    /// Flat rows for the shared virtual-list renderer.
    #[must_use]
    pub fn rows(&self) -> &[DisplayRow] {
        &self.rows
    }

    /// The original numbered source row, retaining text, cuts and hunk keys.
    #[must_use]
    pub fn source(&self, row: SourceRow) -> Option<&DiffRow> {
        self.source.get(row.0)
    }

    /// Canonical review identity is allocated only on interaction, not for
    /// every painted cell of a long file. A blank cell has no cell to pass.
    #[must_use]
    pub fn review_key(&self, cell: DiffCell) -> Option<ReviewKey> {
        if cell.stamp != self.stamp {
            return None;
        }
        let DiffRow::Line {
            key: DiffKey::Line { .. },
            ..
        } = self.source(cell.source)?
        else {
            return None;
        };
        Some(ReviewKey {
            path: self.path.clone(),
            side: cell.side,
            line: cell.number?,
        })
    }
}

fn cell(row: &DiffRow, index: usize, side: ReviewSide, stamp: FileStamp) -> DiffCell {
    let number = match row {
        DiffRow::Line { old, new, .. } => match side {
            ReviewSide::Old => *old,
            ReviewSide::New => *new,
        },
        DiffRow::Hunk { .. } | DiffRow::Cut { .. } => None,
    };
    DiffCell {
        source: SourceRow(index),
        stamp,
        side,
        number: number.and_then(SourceLine::new),
    }
}

fn unified(source: &[DiffRow], stamp: FileStamp) -> Vec<DisplayRow> {
    source
        .iter()
        .enumerate()
        .map(|(index, row)| match row {
            DiffRow::Hunk { .. } => DisplayRow::Header(SourceRow(index)),
            DiffRow::Cut { .. }
            | DiffRow::Line {
                line: LineKind::NoNewline,
                ..
            } => DisplayRow::Notice(SourceRow(index)),
            DiffRow::Line {
                line: LineKind::Removed,
                ..
            } => DisplayRow::Unified(cell(row, index, ReviewSide::Old, stamp)),
            DiffRow::Line {
                line: LineKind::Added | LineKind::Context,
                ..
            } => DisplayRow::Unified(cell(row, index, ReviewSide::New, stamp)),
        })
        .collect()
}

fn split(source: &[DiffRow], stamp: FileStamp) -> Vec<DisplayRow> {
    let mut rows = Vec::with_capacity(source.len());
    let mut pending = Replacement::default();
    for (index, row) in source.iter().enumerate() {
        match row {
            DiffRow::Line {
                line: LineKind::Removed,
                ..
            } => pending.old.push(cell(row, index, ReviewSide::Old, stamp)),
            DiffRow::Line {
                line: LineKind::Added,
                ..
            } => pending.new.push(cell(row, index, ReviewSide::New, stamp)),
            DiffRow::Line {
                line: LineKind::Context,
                ..
            } => {
                pending.flush(&mut rows);
                rows.push(DisplayRow::Split {
                    old: Some(cell(row, index, ReviewSide::Old, stamp)),
                    new: Some(cell(row, index, ReviewSide::New, stamp)),
                });
            }
            DiffRow::Hunk { .. } => {
                pending.flush(&mut rows);
                rows.push(DisplayRow::Header(SourceRow(index)));
            }
            DiffRow::Cut { .. }
            | DiffRow::Line {
                line: LineKind::NoNewline,
                ..
            } => {
                pending.flush(&mut rows);
                rows.push(DisplayRow::Notice(SourceRow(index)));
            }
        }
    }
    pending.flush(&mut rows);
    rows
}

#[derive(Default)]
struct Replacement {
    old: Vec<DiffCell>,
    new: Vec<DiffCell>,
}

impl Replacement {
    fn flush(&mut self, rows: &mut Vec<DisplayRow>) {
        let count = self.old.len().max(self.new.len());
        rows.extend((0..count).map(|index| DisplayRow::Split {
            old: self.old.get(index).copied(),
            new: self.new.get(index).copied(),
        }));
        self.old.clear();
        self.new.clear();
    }
}

#[cfg(test)]
mod tests;
