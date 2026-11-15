//! A theme's radio tile and token-only miniature, shared by Appearance
//! and onboarding. Selection and contribution identity stay with the caller.
//!
//! [Decision Log]
//! - Purpose: one ThemePicker drawing for built-in and contributed palettes.
//! - Constraint: the kit cannot know the view preference or a plugin/host id.
//! - Choice: Single/Split token previews and Chosen/Idle state; the caller
//!   supplies its label, id and click action. Reuse ordinary GPUI elements.
//! - Reference: Paseo appearance/theme-picker.tsx:203-303. Its small12 label
//!   replaces the old area's documented Body14 departure when consumed.
//! - Cost: grid composition is explicit so no identity leaks into this control.

use gpui::prelude::*;
use gpui::{Div, ElementId, Hsla, Pixels, SharedString, Stateful, div, px, relative};

use crate::{Tokens, Type, TypeExt as _};

/// Whether this radio tile is the chosen option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The current choice, with its accent border.
    Chosen,
    /// Another choice, with a surface highlight under the pointer.
    Idle,
}

/// The miniature's colours, independent of preference or contribution types.
#[derive(Debug, Clone, Copy)]
pub enum Preview<'a> {
    /// One concrete palette.
    Single(&'a Tokens),
    /// An adaptive choice: the left light half and the right dark half.
    Split {
        /// Colours on the left.
        light: &'a Tokens,
        /// Colours on the right.
        dark: &'a Tokens,
    },
}

/// The reference's responsive grid fractions; the caller chooses the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Columns {
    /// Two columns in the compact grid.
    Two,
    /// Four columns in the wide grid.
    Four,
}

impl Columns {
    /// The source's medium breakpoint is viewport-wide, not card-wide.
    #[must_use]
    pub fn at(width: Pixels) -> Self {
        const MEDIUM: f32 = 600.0;
        if width < px(MEDIUM) {
            Self::Two
        } else {
            Self::Four
        }
    }
}

const HALF_GAP: f32 = 6.0;
const TILE_INSET: f32 = 4.0;
const TILE_RADIUS: f32 = 8.0;
const FRAME_RADIUS: f32 = 6.0;
const ASPECT: f32 = 16.0 / 9.0;
const SIDEBAR: f32 = 36.0;
const SIDE_INSET: f32 = 6.0;
const CONTENT_INSET: f32 = 8.0;
const LINE_GAP: f32 = 5.0;
const LINE_HEIGHT: f32 = 4.0;
const LINE_RADIUS: f32 = 2.0;
const TITLE_HEIGHT: f32 = 5.0;
const BUBBLE_HEIGHT: f32 = 10.0;
const BUBBLE_RADIUS: f32 = 5.0;
const MUTED_OPACITY: f32 = 0.6;

/// The ThemePicker grid's half-gap cancellation, without its choices.
#[must_use]
pub fn grid() -> Div {
    div().flex().flex_row().flex_wrap().m(px(-HALF_GAP))
}

/// Place a caller-owned interactive tile in the reference's grid cell.
#[must_use]
pub fn cell(tile: impl IntoElement, columns: Columns) -> Div {
    let fraction = match columns {
        Columns::Two => 0.5,
        Columns::Four => 0.25,
    };
    div().w(relative(fraction)).p(px(HALF_GAP)).child(tile)
}

/// A radio tile's one drawing. The opaque UI id retains hover state;
/// attach an action to the returned frame, then put it in [`cell`].
/// Padding outside the frame is not a click target.
#[must_use]
pub fn theme_tile(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    state: State,
    preview: Preview<'_>,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    frame(label, state, preview, tokens)
        .id(id)
        .when(state == State::Idle, |tile| {
            tile.hover(move |style| style.bg(neutral.surface2))
        })
}

// Keep the actual layout body a Div so its child bounds can be inspected
// before GPUI wraps it in the opaque stateful hover/click element.
fn frame(
    label: impl Into<SharedString>,
    state: State,
    preview: Preview<'_>,
    tokens: &Tokens,
) -> Div {
    let neutral = tokens.neutral;
    let border = match state {
        State::Chosen => neutral.accent,
        State::Idle => gpui::transparent_black(),
    };
    div()
        .flex()
        .flex_col()
        .gap(px(TILE_INSET))
        .p(px(TILE_INSET))
        .rounded(px(TILE_RADIUS))
        .border_2()
        .border_color(border)
        .cursor_pointer()
        .child(
            div()
                .rounded(px(FRAME_RADIUS))
                .border_1()
                .border_color(neutral.border)
                .overflow_hidden()
                .child(preview_element(preview)),
        )
        .child(
            div()
                .px(px(TILE_INSET))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .type_(Type::Meta)
                .text_color(neutral.foreground)
                .child(label.into()),
        )
}

fn preview_element(preview: Preview<'_>) -> Div {
    match preview {
        Preview::Single(tokens) => picture(tokens),
        Preview::Split { light, dark } => div().relative().w_full().child(picture(light)).child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(0.5))
                .w(relative(0.5))
                .overflow_hidden()
                .child(
                    div()
                        .w(relative(2.0))
                        .ml(relative(-1.0))
                        .child(picture(dark)),
                ),
        ),
    }
}

fn line(width: f32, height: f32, colour: Hsla) -> Div {
    div()
        .w(relative(width))
        .h(px(height))
        .rounded(px(LINE_RADIUS))
        .bg(colour)
}

fn picture(tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    let muted = neutral.foreground_muted.opacity(MUTED_OPACITY);
    let sidebar = div()
        .w(px(SIDEBAR))
        .h_full()
        .flex()
        .flex_col()
        .gap(px(LINE_GAP))
        .px(px(SIDE_INSET))
        .py(px(CONTENT_INSET))
        .bg(neutral.surface_sidebar)
        .children([1.0, 0.6, 0.8].map(|width| line(width, LINE_HEIGHT, muted)));
    let main = div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(LINE_GAP))
        .p(px(CONTENT_INSET))
        .child(line(0.55, TITLE_HEIGHT, neutral.foreground))
        .children([0.9, 0.65].map(|width| line(width, LINE_HEIGHT, muted)))
        .child(
            div()
                .mt_auto()
                .self_end()
                .w(relative(0.45))
                .h(px(BUBBLE_HEIGHT))
                .rounded(px(BUBBLE_RADIUS))
                .bg(neutral.accent),
        );
    div()
        .w_full()
        .aspect_ratio(ASPECT)
        .flex()
        .flex_row()
        .overflow_hidden()
        .bg(neutral.surface0)
        .child(sidebar)
        .child(main)
}

#[cfg(test)]
mod tests;
