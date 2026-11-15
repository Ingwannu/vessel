//! A combobox's rows (split from `picker.rs`, which keeps the pill, the
//! state and the list's frame, at the file budget, docs/QUALITY.md M3):
//! the rows under their headings, each row's leading, words and handlers.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{AnyElement, Context, Div, ElementId, MouseButton, SharedString, div, px};

use super::{
    ComboHandlers, ComboHeader, ComboRow, ComboboxState, Leading, glyph, initial_tile, one_line,
};
use crate::assets::Glyph;
use crate::fonts::Type;
use crate::theme::Tokens;
use crate::widgets::TypeExt as _;

/// The rows, under their headings, or the words for none.
pub(super) fn rows_list<V: 'static>(
    header: &ComboHeader,
    rows: &[ComboRow],
    state: &ComboboxState,
    handlers: &ComboHandlers<V>,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let listing = Listing {
        column: leading_column(rows),
        highlighted: state.highlight_in(rows),
        state,
    };
    // The rows scroll inside the panel's height; the title and the search
    // row above them stay (`min_h_0` lets the list be shorter than its rows).
    let mut list = div()
        .id("combobox-rows")
        .flex_shrink(1.0)
        .min_h_0()
        .flex()
        .flex_col()
        .py(px(4.0))
        .overflow_y_scroll();
    if rows.is_empty() {
        list = list.child(
            div()
                .px(px(12.0))
                .py(px(8.0))
                .text_color(neutral.foreground_muted)
                .child(header.empty.clone()),
        );
    }
    for (index, row) in rows.iter().enumerate() {
        if let Some(heading) = &row.heading {
            list = list.child(
                div()
                    .px(px(12.0))
                    .pt(px(8.0))
                    .pb(px(4.0))
                    .type_(Type::Meta)
                    .text_color(neutral.foreground_muted)
                    .child(heading.clone()),
            );
        }
        list = list.child(combo_row(
            index, row, listing, handlers, compact, tokens, cx,
        ));
    }
    list.into_any_element()
}

/// How wide a row's leading is, for keeping its column on rows without
/// one: a glyph, a mode's mark and a project's tile are 16, a machine's dot
/// and a small glyph 14. An agent's mark has none: an agent list gives
/// every row one (the bot when the agent is unknown), and its artwork's
/// width is the agent's.
const fn leading_width(leading: &Leading) -> Option<f32> {
    match leading {
        Leading::Glyph(_) | Leading::Mark(_) | Leading::Initial(_) => Some(16.0),
        Leading::Dot(_) => Some(14.0),
        Leading::SmallGlyph(_) => Some(SMALL_ROW_GLYPH),
        Leading::AgentMark(_) => None,
    }
}

/// The leading column a list keeps on every row, if any.
///
/// When any row leads with a mark, a row without one keeps the mark's room,
/// so every label starts at one left edge. Paseo never meets the case in
/// its permissions list (its manifest gives every mode it lists a mark);
/// Vessel does (Codex's `read-only` beside its marked modes), so this is a
/// stated difference: the alternative, a label starting in the mark column,
/// puts one row of the list out of line.
pub(super) fn leading_column(rows: &[ComboRow]) -> Option<f32> {
    rows.iter()
        .filter_map(|row| row.leading.as_ref().and_then(leading_width))
        .reduce(f32::max)
}

/// The header's note ([`ComboHeader::note`]): muted, small, wrapping,
/// above the rows.
pub(super) fn note_row(note: SharedString, tokens: &Tokens) -> Div {
    div()
        .flex_none()
        .px(px(12.0))
        .pt(px(8.0))
        .pb(px(4.0))
        .type_(Type::Meta)
        .text_color(tokens.neutral.foreground_muted)
        .whitespace_normal()
        .child(note)
}

/// The title row: a way back when it leads back, the title in 14 medium.
pub(super) fn title_row<V: 'static>(
    title: SharedString,
    back: bool,
    state: &ComboboxState,
    handlers: &ComboHandlers<V>,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let leading = state.header_leading.clone();
    let lease = state.render_lease();
    let get = handlers.state;
    let mut row = div()
        .id("combobox-title")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(12.0))
        .py(px(10.0))
        .border_b_1()
        .border_color(neutral.border)
        .type_(Type::Label)
        .text_color(neutral.foreground);
    if back {
        row = row.child(glyph(Glyph::ChevronLeft, 16.0, neutral.foreground_muted));
        if let Some(go_back) = handlers.back.clone() {
            row = row
                .cursor_pointer()
                .hover(move |style| style.bg(neutral.surface1))
                .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
                    if !get(view).accepts(&lease) {
                        cx.stop_propagation();
                        return;
                    }
                    go_back(view, window, cx);
                    cx.notify();
                }));
        }
    }
    if let Some(leading) = leading {
        let mark = row_leading(
            leading,
            false,
            crate::agent_mark::Size::Sixteen,
            crate::agent_mark::Space::Lockup,
            tokens,
        );
        row = row.gap(px(crate::theme::spacing::S2)).child(mark);
    }
    // Paseo's sheet header uses one line with an ellipsis; constrain the
    // text slot so a long provider name cannot run through the panel edge.
    row.child(div().flex_1().min_w_0().truncate().child(title))
        .into_any_element()
}

/// A row's words: its label, its detail on the same line, its note under
/// them.
fn row_words(row: &ComboRow, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    let label = div()
        .flex_shrink_0()
        .max_w_full()
        .truncate()
        .text_color(neutral.foreground)
        .child(one_line(&row.label));
    let mut words = div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .flex()
        .flex_row()
        .items_baseline()
        .gap(px(8.0))
        .child(label);
    if let Some(note) = &row.note {
        // The label, and under it what the option is, wrapping.
        words = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(words)
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(neutral.foreground_muted)
                    .whitespace_normal()
                    .child(note.clone()),
            );
    }
    if let Some(detail) = &row.detail {
        words = words.child(
            div()
                .flex_shrink(1.0)
                .min_w_0()
                .truncate()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .child(one_line(detail)),
        );
    }
    words
}

/// Existing picker rows keep their 16-point leading glyph.
pub(super) const ROW_GLYPH: f32 = 16.0;
/// Paseo’s branch-switcher supplies its own 14-point leading slot.
pub(super) const SMALL_ROW_GLYPH: f32 = 14.0;

/// What leads a row.
fn row_leading(
    leading: Leading,
    compact: bool,
    mark_size: crate::agent_mark::Size,
    space: crate::agent_mark::Space,
    tokens: &Tokens,
) -> AnyElement {
    let neutral = tokens.neutral;
    match leading {
        Leading::Glyph(which) => {
            glyph(which, ROW_GLYPH, neutral.foreground_muted).into_any_element()
        }
        Leading::Mark(which) => glyph(which, ROW_GLYPH, neutral.foreground).into_any_element(),
        // BranchSwitcher's explicit 14-point slot must not resize every
        // existing 16-point picker glyph merely because this caller needs it.
        Leading::SmallGlyph(which) => {
            glyph(which, SMALL_ROW_GLYPH, neutral.foreground_muted).into_any_element()
        }
        Leading::AgentMark(art) => {
            let mark = crate::agent_mark::agent_mark(art, mark_size, space, tokens);
            // The minimum OpenCode box occupies the row's existing vertical
            // padding too. Top alignment keeps its edges on whole pixels
            // when the Body line is 21 points high; centring would add half
            // a pixel. Its ink stays inside the content band;
            // the 8-point clear space above/below uses the row's 8-point inset.
            if art == Some(crate::agent_mark::OPENCODE) {
                mark.when(!compact, Styled::self_start)
                    .my(px(crate::theme::spacing::S2.mul_add(-1.0, 0.0)))
                    .into_any_element()
            } else {
                mark.into_any_element()
            }
        }
        Leading::Dot(color) => div()
            .size(px(14.0))
            .flex()
            .items_center()
            .justify_center()
            .child(div().size(px(6.0)).rounded_full().bg(color))
            .into_any_element(),
        Leading::Initial(initial) => initial_tile(initial, tokens).into_any_element(),
    }
}

/// What a row takes from its list: the mark column every row keeps, the
/// highlighted row, and the list's state (its rows' tooltips).
#[derive(Clone, Copy)]
pub(super) struct Listing<'a> {
    pub(super) column: Option<f32>,
    pub(super) highlighted: Option<usize>,
    pub(super) state: &'a ComboboxState,
}

pub(super) fn combo_row<V: 'static>(
    index: usize,
    row: &ComboRow,
    listing: Listing<'_>,
    handlers: &ComboHandlers<V>,
    compact: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, V>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let Listing {
        column,
        highlighted,
        state,
    } = listing;
    let highlighted = highlighted == Some(index);
    let wash = neutral.surface1;
    // Paseo's option (`ComboboxItem`): its label, then its description on
    // the same line, each one line (`numberOfLines={1}`); the description
    // gives way first and ends in an ellipsis, then the label does. A row is
    // one line high whatever an agent writes, so rows never run into each
    // other.
    // Paseo's label does not shrink (`flexShrink: 0`): the description
    // gives way first, all of it, before the label loses a letter. Here it
    // is also at most the row's width, so a name longer than the row ends
    // in an ellipsis. (A small shrink factor is not enough: any shrink at
    // all cuts the label, "Workspace acce…" beside a whole description.)
    // Disabled text follows Paseo's opacity, but official artwork must
    // retain its owner's approved colours instead of being faded with it.
    let fade_text = row.disabled && matches!(row.leading, Some(Leading::AgentMark(Some(_))));
    let words = row_words(row, tokens).when(fade_text, |words| words.opacity(0.55));
    let mut element = div()
        .id(ElementId::NamedInteger(
            "combobox-row".into(),
            u64::try_from(index).unwrap_or(u64::MAX),
        ))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .flex_shrink_0()
        .min_h(px(if compact { 44.0 } else { 36.0 }))
        .px(px(12.0))
        .py(px(8.0))
        .when(row.disabled && !fade_text, |element| element.opacity(0.55))
        .when(highlighted && !row.disabled, |element| element.bg(wash))
        .map(|element| match (row.leading.clone(), column) {
            (Some(leading), _) => element.child(row_leading(
                leading,
                compact,
                crate::agent_mark::Size::Fourteen,
                crate::agent_mark::Space::Row,
                tokens,
            )),
            // The list's mark column, kept empty ([`leading_column`]).
            (None, Some(width)) => element.child(div().flex_none().w(px(width))),
            (None, None) => element,
        })
        .child(words)
        // Paseo's trailing slot: the check, then the row's own words (a
        // value at most 45% of the row, one line).
        .when(row.checked, |element| {
            element.child(glyph(Glyph::Check, 16.0, neutral.foreground_muted).when(fade_text, |check| check.opacity(0.55)))
        })
        .children(row.trailing.clone().map(|trailing| {
            div()
                .flex_shrink(1.0)
                .min_w_0()
                .max_w(gpui::relative(0.45))
                .truncate()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .when(fade_text, |trailing| trailing.opacity(0.55))
                .child(trailing)
        }))
        .when(row.opens, |element| {
            element.child(glyph(Glyph::ChevronRight, 14.0, neutral.foreground_muted).when(fade_text, |chevron| chevron.opacity(0.55)))
        })
        .children(row.switch.map(|on| {
            let control = crate::settings::switch(
                ElementId::NamedInteger(
                    "combobox-switch".into(),
                    u64::try_from(index).unwrap_or(u64::MAX),
                ),
                on,
                !row.disabled,
                compact,
                0,
                tokens,
            );
            if fade_text {
                div().opacity(0.55).child(control).into_any_element()
            } else {
                control.into_any_element()
            }
        }));
    if let Some(whole) = &row.tooltip {
        element = state.tips.on_code(
            element,
            &state.tip_places.get(index),
            whole.clone(),
            compact,
        );
    }
    if !row.disabled {
        element = enabled_row(element, index, state, handlers, wash, cx);
    }
    element.into_any_element()
}

/// Bind one enabled row through the same captured opening; layout stays above.
fn enabled_row<V: 'static>(
    element: gpui::Stateful<Div>,
    index: usize,
    state: &ComboboxState,
    handlers: &ComboHandlers<V>,
    wash: gpui::Hsla,
    cx: &mut Context<'_, V>,
) -> gpui::Stateful<Div> {
    let chose = Rc::clone(&handlers.chose);
    let get = handlers.state;
    let lease = state.render_lease();
    element
        .cursor_pointer()
        .hover(move |style| style.bg(wash))
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .on_click(cx.listener(move |view: &mut V, _event, window, cx| {
            if !get(view).accepts(&lease) {
                cx.stop_propagation();
                return;
            }
            chose(view, index, window, cx);
            get(view).restore_choice_if_current(&lease, window, cx);
            cx.notify();
        }))
}
