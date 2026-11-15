//! A thin readonly unified-diff preview, not an editor or highlighter.
//!
//! [Decision Log]
//! - Purpose: match Paseo appearance-preview.tsx with local typography drafts.
//! - Constraint: preserve the existing classifier and `CodeStyles` resolver;
//!   the kit accepts only semantic byte runs and no view/plugin identities.
//! - Choice: one `StyledText` row renderer, validated UTF-8 ranges and local
//!   `FontStack`/`FontSize`. No input entity, caret, history or global mutation.
//! - Reference: card surface1/border1/r8/py8, rows px12, caption px12/pb8;
//!   full-row source tints and diff marker text colours live in `theme::output`.
use crate::{
    Tokens,
    code_editor::CodeStyles,
    fonts::{CODE_LINE_RATIO, CONTENT_LINE_RATIO, FontSize, FontStack},
    output::Tone,
    syntax::CodeRun,
    theme::output::PreviewDiff,
};
use gpui::prelude::*;
use gpui::{
    Div, ElementId, Font, FontFeatures, FontStyle, FontWeight, SharedString, StyledText, div, px,
};
use std::sync::Arc;

const INSET_Y: f32 = 8.0;
const INSET_X: f32 = 12.0;
const RADIUS: f32 = 8.0;
const MARKER_BYTES: usize = 2;

/// Local resolved typography. Drawing it never changes a global preference.
#[derive(Debug, Clone)]
pub struct PreviewTypography {
    /// Prose inherits the chosen interface family.
    pub content: FontStack,
    /// The valid code-family draft or its saved fallback.
    pub code: FontStack,
    /// Absolute content-size draft or its saved fallback.
    pub content_size: FontSize,
    /// Absolute code-size draft or its saved fallback.
    pub code_size: FontSize,
}

/// A prepared row. Prefixing and range validation occur once at construction,
/// never while painting an unchanged preview.
#[derive(Debug, Clone)]
pub struct PreviewRow {
    id: ElementId,
    text: SharedString,
    tone: Tone,
    runs: Arc<[CodeRun]>,
}

/// A caller's byte runs cannot be safely handed to a text renderer.
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    /// One row cannot contain a newline.
    #[error("a preview row contains a newline")]
    Newline,
    /// Runs must be ordered, non-overlapping and within the source row.
    #[error("a preview syntax run is outside or overlaps its row")]
    Range,
    /// A byte boundary cuts a Unicode scalar.
    #[error("a preview syntax run splits a Unicode character")]
    Encoding,
    /// The prepared row would exceed an addressable string size.
    #[error("a preview row exceeds the addressable size")]
    Length,
    /// Allocation failed before any row was prepared.
    #[error("preparing a preview row: {0}")]
    Capacity(#[from] std::collections::TryReserveError),
}

impl PreviewRow {
    /// Prepare source code with its unified-diff marker and semantic runs.
    /// Existing `output::Tone` supplies context/add/remove meaning.
    ///
    /// # Errors
    /// Refuses multiline text and malformed/overlapping UTF-8 byte runs.
    pub fn new(
        id: impl Into<ElementId>,
        source: &str,
        tone: Tone,
        mut runs: Vec<CodeRun>,
    ) -> Result<Self, PreviewError> {
        if source.contains(['\n', '\r']) {
            return Err(PreviewError::Newline);
        }
        let mut end = 0;
        for run in &mut runs {
            if run.range.start < end
                || run.range.start > run.range.end
                || run.range.end > source.len()
            {
                return Err(PreviewError::Range);
            }
            if !source.is_char_boundary(run.range.start) || !source.is_char_boundary(run.range.end)
            {
                return Err(PreviewError::Encoding);
            }
            end = run.range.end;
            run.range.start = run
                .range
                .start
                .checked_add(MARKER_BYTES)
                .ok_or(PreviewError::Length)?;
            run.range.end = run
                .range
                .end
                .checked_add(MARKER_BYTES)
                .ok_or(PreviewError::Length)?;
        }
        let prefix = match tone {
            Tone::Plain => "  ",
            Tone::Added => "+ ",
            Tone::Removed => "- ",
        };
        let mut text = String::new();
        text.try_reserve(
            source
                .len()
                .checked_add(MARKER_BYTES)
                .ok_or(PreviewError::Length)?,
        )?;
        text.push_str(prefix);
        text.push_str(source);
        Ok(Self {
            id: id.into(),
            text: text.into(),
            tone,
            runs: runs.into(),
        })
    }
}

/// Draw the source's readonly preview with the existing shared style resolver.
/// Rows are prepared outside paint; text deliberately does not wrap, matching
/// the reference's preformatted rows and clipped card.
#[must_use]
pub fn code_preview(
    caption: impl Into<SharedString>,
    rows: &[PreviewRow],
    typography: &PreviewTypography,
    tokens: &Tokens,
) -> Div {
    let neutral = tokens.neutral;
    let content_size = f32::from(typography.content_size.get());
    let code_size = f32::from(typography.code_size.get());
    let styles = CodeStyles::of(tokens.scheme);
    let paint = PreviewDiff::of(tokens.scheme);
    div()
        .w_full()
        .flex()
        .flex_col()
        .py(px(INSET_Y))
        .rounded(px(RADIUS))
        .border_1()
        .border_color(neutral.border)
        .bg(neutral.surface1)
        .overflow_hidden()
        .child(
            div()
                .px(px(INSET_X))
                .pb(px(INSET_Y))
                .font(font(&typography.content))
                .text_size(px(content_size))
                .line_height(px(content_size.mul_add(CONTENT_LINE_RATIO, 0.0).round()))
                .text_color(neutral.foreground)
                .child(caption.into()),
        )
        .children(
            rows.iter()
                .map(|row| row_element(row, &styles, paint, typography, code_size, tokens)),
        )
}

fn row_element(
    row: &PreviewRow,
    styles: &CodeStyles,
    paint: PreviewDiff,
    typography: &PreviewTypography,
    size: f32,
    tokens: &Tokens,
) -> gpui::Stateful<Div> {
    let (marker, background) = match row.tone {
        Tone::Plain => (tokens.neutral.foreground_muted, gpui::transparent_black()),
        Tone::Added => (paint.addition, paint.added_tint),
        Tone::Removed => (paint.deletion, paint.removed_tint),
    };
    let marker = (
        0..MARKER_BYTES,
        gpui::HighlightStyle {
            color: Some(marker),
            ..Default::default()
        },
    );
    let highlights = std::iter::once(marker).chain(
        row.runs
            .iter()
            .map(|run| (run.range.clone(), styles.highlight(run.style))),
    );
    div()
        .id(row.id.clone())
        .w_full()
        .px(px(INSET_X))
        .bg(background)
        .font(font(&typography.code))
        .text_size(px(size))
        .line_height(px(size.mul_add(CODE_LINE_RATIO, 0.0).round()))
        .text_color(tokens.neutral.foreground)
        .whitespace_nowrap()
        .child(StyledText::new(row.text.clone()).with_highlights(highlights))
}
fn font(stack: &FontStack) -> Font {
    Font {
        family: stack.family(),
        features: FontFeatures::default(),
        fallbacks: Some(stack.fallbacks()),
        weight: FontWeight::NORMAL,
        style: FontStyle::Normal,
    }
}
#[cfg(test)]
mod tests;
