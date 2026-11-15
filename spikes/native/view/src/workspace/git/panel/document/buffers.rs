//! Selectable editor input from the already packed diff, without another lexer.
//!
//! [Decision Log]
//! - Purpose: a buffer row and its source number must never drift on replacement
//!   runs, blank split cells, headers, trailing empty lines or truncated hunks.
//! - Existing assets: `DiffDocument` owns pairing and source identity. The kit
//!   owns text input, selection and rendering; this projection has no UI types.
//! - Choice: snapshot each packed column once. Keep semantic metadata alongside
//!   the joined buffer; review keys are still allocated only on interaction.
//! - Cost: joining text allocates one bounded buffer per column. The caller must
//!   apply its body-cache budget, not retain every file's editor indefinitely.
use super::*;

/// A row's semantic paint, independent of a toolkit colour or editor engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferKind {
    /// Unmodified source.
    Context,
    /// Added source.
    Added,
    /// Removed source.
    Removed,
    /// A split-side filler, with no source or review identity.
    Empty,
    /// A hunk header retained in selection/copy, without a source number.
    Header,
    /// A missing-newline marker or an explicit truncated-hunk boundary.
    Notice,
}

/// Metadata for one engine buffer row; line numbers are not part of its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferLine {
    /// The existing source identity; never a display index posing as authority.
    pub cell: Option<DiffCell>,
    /// Number on the selected old/new side.
    pub number: Option<SourceLine>,
    /// Semantic painting, projected into kit tokens by the screen.
    pub kind: BufferKind,
    /// A line or hunk was cut; the consumer must retain its truncation notice.
    pub cut: bool,
}
impl BufferLine {
    const fn empty() -> Self {
        Self {
            cell: None,
            number: None,
            kind: BufferKind::Empty,
            cut: false,
        }
    }
}

/// Text and presentation rows created together, including a final empty row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferColumn {
    /// Exact joined row text, without diff prefixes or presentation numbers.
    pub text: Arc<str>,
    /// Exactly one entry per buffer line, including a trailing newline's row.
    pub lines: Arc<[BufferLine]>,
}

/// Split buffers do not interleave old and new text during selection/copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferColumns {
    /// One ordered diff buffer.
    Unified(BufferColumn),
    /// One old-side and one new-side buffer with identical row counts.
    Split {
        /// Original-side text, without interleaved additions.
        old: BufferColumn,
        /// Modified-side text, without interleaved removals.
        new: BufferColumn,
    },
}

#[derive(Default)]
struct Column {
    text: String,
    lines: Vec<BufferLine>,
}
impl Column {
    fn push(&mut self, text: &str, line: BufferLine) {
        if !self.lines.is_empty() {
            self.text.push('\n');
        }
        self.text.push_str(text);
        self.lines.push(line);
    }
    fn finish(mut self) -> BufferColumn {
        // An empty engine still has one buffer row. This placeholder is never
        // given a source identity and does not mean the file was unchanged.
        if self.lines.is_empty() {
            self.lines.push(BufferLine::empty());
        }
        BufferColumn {
            text: self.text.into(),
            lines: self.lines.into(),
        }
    }
}

/// A peer's one wire line cannot become several unnumbered engine rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BufferError {
    /// The original document remains available for diagnostics; no partial
    /// buffer with misaligned numbering/selection is admitted.
    #[error("diff source row {row:?} contains an embedded line break")]
    EmbeddedNewline {
        /// Original wire-row identity, retained for exact diagnostics.
        row: SourceRow,
    },
}

impl DiffDocument {
    /// Snapshot the packed text. Headers remain copyable; explicit cut markers
    /// occupy an unnumbered empty row and retain `cut` for an external notice.
    /// No marker is silently promoted into source/review authority.
    ///
    /// # Errors
    /// A wire line or hunk section containing an embedded newline is refused,
    /// retaining the original document rather than inventing source numbers.
    pub fn buffers(&self) -> Result<BufferColumns, BufferError> {
        for (index, row) in self.source.iter().enumerate() {
            let newline = match row {
                DiffRow::Line { text, .. } => text.contains('\n'),
                DiffRow::Hunk {
                    header, section, ..
                } => {
                    header.contains('\n')
                        || section
                            .as_ref()
                            .is_some_and(|section| section.contains('\n'))
                }
                DiffRow::Cut { .. } => false,
            };
            if newline {
                return Err(BufferError::EmbeddedNewline {
                    row: SourceRow(index),
                });
            }
        }
        let mut old = Column::default();
        let mut new = Column::default();
        for row in &self.rows {
            match *row {
                DisplayRow::Unified(cell) => self.push_cell(&mut old, Some(cell)),
                DisplayRow::Split {
                    old: left,
                    new: right,
                } => {
                    self.push_cell(&mut old, left);
                    self.push_cell(&mut new, right);
                }
                DisplayRow::Header(source) | DisplayRow::Notice(source) => {
                    self.push_marker(&mut old, source);
                    self.push_marker(&mut new, source);
                }
            }
        }
        Ok(if self.layout == Layout::Split {
            BufferColumns::Split {
                old: old.finish(),
                new: new.finish(),
            }
        } else {
            BufferColumns::Unified(old.finish())
        })
    }

    fn push_cell(&self, column: &mut Column, cell: Option<DiffCell>) {
        let Some(cell) = cell else {
            column.push("", BufferLine::empty());
            return;
        };
        if let Some(DiffRow::Line {
            text, line, cut, ..
        }) = self.source(cell.source())
        {
            let kind = match line {
                LineKind::Context => BufferKind::Context,
                LineKind::Added => BufferKind::Added,
                LineKind::Removed => BufferKind::Removed,
                LineKind::NoNewline => BufferKind::Notice,
            };
            column.push(
                text,
                BufferLine {
                    cell: Some(cell),
                    number: cell.number(),
                    kind,
                    cut: *cut,
                },
            );
        }
    }

    fn push_marker(&self, column: &mut Column, source: SourceRow) {
        match self.source(source) {
            Some(DiffRow::Hunk {
                header, section, ..
            }) => {
                let header = section
                    .as_ref()
                    .map_or_else(|| header.clone(), |section| format!("{header} {section}"));
                column.push(
                    &header,
                    BufferLine {
                        kind: BufferKind::Header,
                        ..BufferLine::empty()
                    },
                );
            }
            Some(DiffRow::Line { text, cut, .. }) => column.push(
                text,
                BufferLine {
                    kind: BufferKind::Notice,
                    cut: *cut,
                    ..BufferLine::empty()
                },
            ),
            Some(DiffRow::Cut { .. }) => column.push(
                "",
                BufferLine {
                    kind: BufferKind::Notice,
                    cut: true,
                    ..BufferLine::empty()
                },
            ),
            None => {}
        }
    }
}
