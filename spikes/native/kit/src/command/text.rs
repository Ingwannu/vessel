//! Command text with semantic prose/code parts. The screen supplies text
//! and byte marks; the kit owns faces, sizes, line layout and truncation.
//!
//! [Decision Log]
//! - Purpose: a status and its path remain one subtitle line without
//!   drawing the path in the interface face or losing search marks.
//! - Constraint: existing callers pass a plain subtitle. A new body type
//!   must preserve their geometry and keep code-size preferences separate.
//! - Choice: plain text or a prose prefix followed by a code path. The
//!   path gets the remaining width, and each part owns its UTF-8 marks.
//! - Cost: a large code preference can increase the subtitle line height;
//!   its natural maximum prevents clipping rather than fixing a UI height.

use std::ops::Range;

use gpui::prelude::*;
use gpui::{Div, HighlightStyle, SharedString, StyledText, div, px};

use crate::fonts::{self, Type};
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

use super::marks;

/// One text part, with matching ranges in its own original UTF-8 bytes.
#[derive(Debug, Clone, Default)]
pub struct CommandText {
    /// The complete label, before layout truncation.
    pub label: SharedString,
    /// Every token or subsequence run to highlight.
    pub marks: Vec<Range<usize>>,
}

impl CommandText {
    /// Text and marks produced by the same view projection.
    #[must_use]
    pub fn new(label: impl Into<SharedString>, marks: Vec<Range<usize>>) -> Self {
        Self {
            label: label.into(),
            marks,
        }
    }
}

/// A one-line subtitle's semantic parts.
#[derive(Debug, Clone)]
pub enum CommandSubtitle {
    /// A plain interface caption (for example a machine's connection).
    Text(CommandText),
    /// A path in the code face, after optional prose and its separator.
    CodePath {
        /// A status and separator, with their own byte ranges.
        prefix: Option<CommandText>,
        /// The path, with ranges measured from the start of this part.
        path: CommandText,
    },
}

impl CommandSubtitle {
    pub(super) fn render(self, tokens: &Tokens) -> Div {
        match self {
            Self::Text(text) => prose(text, tokens).truncate(),
            Self::CodePath { prefix, path } => div()
                .flex()
                .flex_row()
                .items_baseline()
                .min_w_0()
                .children(prefix.map(|text| prose(text, tokens).flex_none().whitespace_nowrap()))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .face(Type::Mono)
                        .text_size(px(fonts::code(12.0)))
                        .line_height(px(fonts::code(16.0)))
                        .text_color(tokens.neutral.foreground_muted)
                        .child(marked(path.label, path.marks, tokens)),
                ),
        }
    }
}

fn prose(text: CommandText, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Meta)
        .line_height(px(fonts::ui(16.0)))
        .text_color(tokens.neutral.foreground_muted)
        .child(marked(text.label, text.marks, tokens))
}

/// All command text uses the same range validation and highlight paint.
pub(super) fn marked(
    text: SharedString,
    matches: Vec<Range<usize>>,
    tokens: &Tokens,
) -> StyledText {
    let style = HighlightStyle {
        color: Some(tokens.neutral.foreground),
        background_color: Some(tokens.neutral.surface3),
        ..HighlightStyle::default()
    };
    let matches = marks::valid(&text, matches);
    StyledText::new(text).with_highlights(matches.into_iter().map(|range| (range, style)))
}
