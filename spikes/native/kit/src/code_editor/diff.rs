//! A mapped read-only diff over the existing Base editor, not a second engine.
//!
//! [Decision Log]
//! - Purpose: cross-row selection/copy/wrap must remain the source editor's.
//! - Constraint: ordinary line numbers do not identify old/new source rows;
//!   a list of styled text rows has no shared input or clipboard semantics.
//! - Choice: reuse `CodeEditor` construction/highlighter and the optional drawing
//!   mapping. Immutable text and row metadata are admitted together, once.
//! - Effect: editing is disabled, selection/copy retained; theme projection is
//!   shared and no parser, file controller, palette or syntax kind is added.
//! - Difference: native gutters measure real labels in the selected code face,
//!   instead of Paseo code-insets.ts's approximate 0.62 font-size multiplier.
//!   The engine's existing six-point gutter gap is retained in this first
//!   control; consumer/reference geometry and border parity remain unverified.
mod document;
mod syntax;
pub use document::DiffDocumentEditor;

use std::num::NonZeroU32;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{AnyElement, App, Context, Window, div};
use gpui_base::input::{BufferRow, Editor, InputLinePaint, InputLinePresentation};

use super::{CodeEditor, LineSyntax};
use crate::{Tokens, Type, TypeExt as _};

/// A semantic diff row; no application identity or review authority is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTone {
    /// Context uses the ordinary source editor surface.
    Context,
    /// Added code and its line number use the success signal.
    Added,
    /// Removed code and its line number use the error signal.
    Removed,
    /// A missing split-side cell keeps an empty surface and number.
    Empty,
    /// A hunk header has no language state from the omitted gap.
    Header,
    /// A structural notice is not source text for the lexer.
    Notice,
}

/// Which original source supplies context-line syntax in this column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffSide {
    /// A split old-side column uses the original source's lexer state.
    Original,
    /// Unified and split new-side context uses the modified source.
    #[default]
    Modified,
}

/// A displayed source number may differ from the engine's buffer row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffLine {
    /// The old/new source line, absent for a filler or unnumbered line.
    pub number: Option<NonZeroU32>,
    /// Selection/copy contain only buffer text, never these presentation facts.
    pub tone: DiffTone,
}

/// Wrapping is presentation, not an edit or a different syntax engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffWrapping {
    /// Keep horizontal scrolling for long code.
    Scroll,
    /// Let the same editor wrap at its viewport width.
    Wrap,
}

/// An engine-buffer row, distinct from a one-based repository line number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffRowIndex(usize);
impl DiffRowIndex {
    /// Inspect only at the diagnostic/render boundary; zero is the first row.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Text and metadata must identify exactly the same buffer rows.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DiffTextError {
    /// Refuse misaligned metadata rather than tinting another source line.
    #[error("the diff has {lines} buffer lines but {rows} presentation rows")]
    RowCount {
        /// A trailing line break creates a final empty buffer row.
        lines: usize,
        /// Metadata supplied by the owner.
        rows: usize,
    },
    /// Structural rows must not display an invented repository line number.
    #[error("non-source diff row {row:?} has a source line number")]
    NonSourceNumber {
        /// Zero-based buffer position, not a repository source line.
        row: DiffRowIndex,
    },
}

/// Immutable validated input; replace the whole editor for another generation.
#[derive(Debug, Clone)]
pub struct DiffText {
    text: Arc<str>,
    rows: Arc<[DiffLine]>,
    sample: gpui::SharedString,
    side: DiffSide,
}
impl DiffText {
    /// Preserve the exact buffer text, including a final empty line.
    ///
    /// # Errors
    /// Refuses metadata not covering every buffer row.
    pub fn new(text: Arc<str>, rows: Arc<[DiffLine]>) -> Result<Self, DiffTextError> {
        let lines = text.split('\n').count();
        if lines != rows.len() {
            return Err(DiffTextError::RowCount {
                lines,
                rows: rows.len(),
            });
        }

        for (index, row) in rows.iter().enumerate() {
            if row.number.is_some()
                && matches!(
                    row.tone,
                    DiffTone::Empty | DiffTone::Header | DiffTone::Notice
                )
            {
                return Err(DiffTextError::NonSourceNumber {
                    row: DiffRowIndex(index),
                });
            }
        }
        let maximum = rows.iter().filter_map(|row| row.number).max();
        let sample = maximum.map_or_else(|| "00".to_owned(), |number| format!("{number:02}"));
        Ok(Self {
            text,
            rows,
            sample: sample.into(),
            side: DiffSide::Modified,
        })
    }

    /// Split old-side columns select their actual source explicitly. Unified
    /// context keeps the default modified-side syntax.
    #[must_use]
    pub const fn with_side(mut self, side: DiffSide) -> Self {
        self.side = side;
        self
    }
}

/// One fixed-generation editor. A clone shares its buffer and selection.
#[derive(Debug, Clone)]
pub struct DiffEditor {
    editor: CodeEditor,
    rows: Arc<[DiffLine]>,
    sample: gpui::SharedString,
}
impl DiffEditor {
    /// Reuse the normal editor's construction and highlighter, then make it
    /// read-only. No mutation callback or replacement buffer escapes this type.
    pub fn new<V: 'static, S: LineSyntax>(
        text: DiffText,
        syntax: S,
        wrapping: DiffWrapping,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let syntax = syntax::DiffSyntax::new(syntax, Arc::clone(&text.rows), text.side);
        let editor = CodeEditor::new(&text.text, syntax, unchanged::<V>, window, cx);
        editor.set_read_only(true, cx);
        let result = Self {
            editor,
            rows: text.rows,
            sample: text.sample,
        };
        result.set_wrapping(wrapping, window, cx);
        result
    }

    /// Reflow the same buffer/selection rather than replacing its engine.
    pub fn set_wrapping(&self, wrapping: DiffWrapping, window: &mut Window, cx: &mut App) {
        let wrap = match wrapping {
            DiffWrapping::Scroll => false,
            DiffWrapping::Wrap => true,
        };
        self.editor.set_wrapping(wrap, window, cx);
    }

    /// Keep keyboard selection and copy available in a read-only surface.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.editor.focus(window, cx);
    }

    /// Paint using the shared syntax projection and current semantic colours.
    pub fn element(&self, tokens: &Tokens, cx: &mut App) -> AnyElement {
        let mut style = super::style(tokens);
        // Paseo diff-document/palette.ts uses terminal.blue for selection,
        // independently of the ordinary source editor's neutral selection.
        let [_, _, _, _, blue, ..] = tokens.terminal.ansi;
        style.selection = gpui::rgb(blue).into();
        style = style.with_line_presentation(Some(Arc::new(Presentation {
            rows: Arc::clone(&self.rows),
            sample: self.sample.clone(),
            added: tokens.status().success.foreground,
            removed: tokens.status().error.foreground,
            muted: tokens.neutral.foreground_muted,
        })));
        self.editor
            .state
            .update(cx, |state, _cx| state.set_editor_style(style));
        div()
            .size_full()
            .type_(Type::Mono)
            .child(Editor::new(&self.editor.state))
            .into_any_element()
    }
}

fn unchanged<V>(_owner: &mut V, _window: &mut Window, _cx: &mut Context<'_, V>) {}

struct Presentation {
    rows: Arc<[DiffLine]>,
    sample: gpui::SharedString,
    added: gpui::Hsla,
    removed: gpui::Hsla,
    muted: gpui::Hsla,
}
impl InputLinePresentation for Presentation {
    fn gutter_sample(&self) -> gpui::SharedString {
        self.sample.clone()
    }
    fn line(&self, row: BufferRow) -> InputLinePaint {
        let Some(row) = self.rows.get(row.index()) else {
            return InputLinePaint::default();
        };
        let (number_color, background) = match row.tone {
            DiffTone::Added => (Some(self.added), Some(self.added.opacity(0.15))),
            DiffTone::Removed => (Some(self.removed), Some(self.removed.opacity(0.1))),
            DiffTone::Context => (Some(self.muted), None),
            DiffTone::Empty | DiffTone::Header | DiffTone::Notice => (None, None),
        };
        InputLinePaint {
            number: row.number.map(|number| number.to_string().into()),
            number_color,
            background,
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "diff/mapping_tests.rs"]
mod mapping_tests;
