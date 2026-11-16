//! Command Center rows and key badges, shared by the palette and
//! keyboard settings. Keeping presentation separate from the frame lets
//! additional match ranges land without changing focus or modal policy.

use std::ops::Range;

use gpui::prelude::*;
use gpui::{Div, ElementId, SharedString, Stateful, div, px, svg};

use crate::assets::Glyph;
use crate::fonts::{self, Type};
use crate::picker::glyph;
use crate::popup;
use crate::theme::{Tokens, spacing};
use crate::widgets::TypeExt as _;

use super::text::marked;
use super::{ACROSS, CommandSubtitle, CommandText};

/// One row's content.
#[derive(Debug, Clone, Default)]
pub struct CommandRow {
    /// Its glyph, muted.
    pub glyph: Option<Glyph>,
    /// Its words.
    pub title: SharedString,
    /// Where the search shows in them, in bytes.
    pub title_mark: Option<Range<usize>>,
    /// A second line, muted.
    pub subtitle: Option<SharedString>,
    /// Where the search shows in it.
    pub subtitle_mark: Option<Range<usize>>,
    /// Its keys, one badge per chord.
    pub keys: Vec<SharedString>,
    /// The choice in force (a check at its end).
    pub checked: bool,
    /// The row the keyboard is on.
    pub active: bool,
}

/// One row: the pointer's hover tints it as the keyboard's active row
/// (it does not move the active row; a click is the screen's).
#[must_use]
pub fn command_row(
    id: impl Into<ElementId>,
    row: CommandRow,
    compact: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let matches = CommandMatches {
        title: row.title_mark.clone().into_iter().collect(),
        subtitle: row.subtitle_mark.clone().into_iter().collect(),
    };
    command_row_matched(id, row, matches, compact, tokens)
}

/// Byte ranges produced by the same matcher that ranked a row. Separate
/// from its content so older callers can still pass their single mark.
#[derive(Debug, Clone, Default)]
pub struct CommandMatches {
    /// Every matching title token, including separated subsequence runs.
    pub title: Vec<Range<usize>>,
    /// Every matching subtitle token.
    pub subtitle: Vec<Range<usize>>,
}

/// A command row whose text parts retain their semantic faces and marks.
#[derive(Debug, Clone, Default)]
pub struct CommandRowParts {
    /// The leading vector glyph.
    pub glyph: Option<Glyph>,
    /// The title and its matching byte ranges.
    pub title: CommandText,
    /// The optional second line: prose or prose followed by a code path.
    pub subtitle: Option<CommandSubtitle>,
    /// The shortcut, one badge per chord.
    pub keys: Vec<SharedString>,
    /// The choice currently in force.
    pub checked: bool,
    /// The keyboard's current row.
    pub active: bool,
}

/// A row with every matching token marked, as Paseo's
/// `findHighlightRanges`. Invalid ranges are skipped, overlapping ranges
/// merged: a stale byte offset must never split a displayed character.
#[must_use]
pub fn command_row_matched(
    id: impl Into<ElementId>,
    row: CommandRow,
    matches: CommandMatches,
    compact: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    command_row_parts(
        id,
        CommandRowParts {
            glyph: row.glyph,
            title: CommandText::new(row.title, matches.title),
            subtitle: row
                .subtitle
                .map(|text| CommandSubtitle::Text(CommandText::new(text, matches.subtitle))),
            keys: row.keys,
            checked: row.checked,
            active: row.active,
        },
        compact,
        tokens,
    )
}

/// Canonical command row renderer. Compatibility entry points convert
/// their plain subtitles into these parts and use the same geometry.
#[must_use]
pub fn command_row_parts(
    id: impl Into<ElementId>,
    row: CommandRowParts,
    compact: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let inset = if compact { spacing::S3 } else { spacing::S1_5 };
    let two = row.subtitle.is_some();
    let title_line = if two { 18.0 } else { 20.0 };
    let title = div()
        .truncate()
        .type_(Type::Body)
        .line_height(px(fonts::ui(title_line)))
        .text_color(neutral.foreground)
        .child(marked(row.title.label, row.title.marks, tokens));
    let subtitle = row.subtitle.map(|subtitle| subtitle.render(tokens));
    let icon = row.glyph.map(|which| {
        div()
            .flex_none()
            .w(px(16.0))
            .h(px(fonts::ui(20.0)))
            .flex()
            .items_center()
            .justify_center()
            // Every command-centre glyph at Paseo's row stroke (2.2).
            .child(
                svg()
                    .path(which.command_weight_path())
                    .size(px(16.0))
                    .flex_shrink_0()
                    .text_color(neutral.foreground_muted),
            )
    });
    let main = div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_row()
        .items_start()
        .gap(px(spacing::S3))
        .children(icon)
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(title)
                .children(subtitle),
        );
    let end = if row.checked {
        Some(glyph(Glyph::Check, 16.0, neutral.foreground).into_any_element())
    } else if row.keys.is_empty() {
        None
    } else {
        Some(key_badges(&row.keys, tokens).into_any_element())
    };
    div()
        .id(id.into())
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(spacing::S3))
        .px(px(ACROSS))
        .py(px(inset))
        .cursor_pointer()
        .when(row.active, |row| row.bg(neutral.surface1))
        .hover(move |style| style.bg(neutral.surface1))
        .child(main)
        .children(end)
}

/// A binding's chords as badges, one per chord, 4 apart.
#[must_use]
pub fn key_badges(keys: &[SharedString], tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .gap(px(spacing::S1))
        .children(keys.iter().map(|key| {
            div()
                .px(px(spacing::S1))
                .py(px(spacing::S0_5))
                .rounded(px(popup::ROW_RADIUS))
                .bg(neutral.surface3.opacity(0.5))
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(key.clone())
        }))
}
