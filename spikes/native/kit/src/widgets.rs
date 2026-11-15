//! Small controls drawn from tokens: the type of a piece of text, a
//! button, a status dot, a column's kicker.

use gpui::prelude::*;
use gpui::{Div, FontStyle, Hsla, SharedString, Stateful, div, px};

use crate::fonts::{self, Type};
use crate::theme::{Tokens, radius};

/// Set an element's type from the scale.
pub trait TypeExt: Styled + Sized {
    /// The size, line height, weight and family of `step`, Pretendard
    /// behind Geist.
    #[must_use]
    fn type_(self, step: Type) -> Self {
        let (size, line, weight) = step.metrics();
        self.font(gpui::Font {
            family: step.family(),
            features: gpui::FontFeatures::default(),
            fallbacks: Some(fonts::fallbacks()),
            weight,
            style: FontStyle::Normal,
        })
        .text_size(px(size))
        .line_height(px(line))
    }

    /// Only the face of `step` (its family and weight, Pretendard behind
    /// it), for an element that sets its own size, or a container whose
    /// rows do. Text whose ancestors set no face has no Pretendard behind
    /// it: the system's fallback draws its Hangul, and at small sizes one
    /// such font split "각" into "가" and "ㄱ" (the folded thought row read
    /// "생가ㄱ"). A family set alone (`font_family`) keeps the fallback
    /// list of the ancestors, so it is safe only inside a face.
    #[must_use]
    fn face(self, step: Type) -> Self {
        let (_size, _line, weight) = step.metrics();
        self.font(gpui::Font {
            family: step.family(),
            features: gpui::FontFeatures::default(),
            fallbacks: Some(fonts::fallbacks()),
            weight,
            style: FontStyle::Normal,
        })
    }
}

impl<E: Styled> TypeExt for E {}

/// Button meanings from Paseo (`default`, `secondary`, `destructive`,
/// `ghost`). Outline retains its existing entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// The primary action, in the app's brand accent.
    Primary,
    /// An ordinary action on surface3.
    Secondary,
    /// A destructive action, with its own foreground independent of the accent.
    Destructive,
    /// A quiet action: transparent, muted text; foreground text on hover.
    Ghost,
}

/// Whether the caller may attach a handler. Disabled buttons keep their words
/// but have no pointer or pressed/hover treatment, as Paseo's Pressable does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// Ready for the caller's handler.
    Enabled,
    /// The caller attaches no handler.
    Disabled,
}

const BUTTON_HEIGHT: gpui::Pixels = px(32.0);
const BUTTON_HORIZONTAL: gpui::Pixels = px(16.0);
const BUTTON_GAP: gpui::Pixels = px(8.0);

#[derive(Debug, Clone, Copy)]
enum ButtonPhase {
    Idle,
    Hovered,
    Pressed,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ButtonPaint {
    fill: Hsla,
    border: Hsla,
    text: Hsla,
    opacity: f32,
}

fn button_paint(kind: ButtonKind, phase: ButtonPhase, tokens: &Tokens) -> ButtonPaint {
    let neutral = tokens.neutral;
    let (fill, text) = match kind {
        ButtonKind::Primary => (neutral.accent, neutral.accent_foreground),
        ButtonKind::Secondary => (neutral.surface3, neutral.foreground),
        ButtonKind::Destructive => (neutral.destructive, neutral.destructive_foreground),
        ButtonKind::Ghost => (
            gpui::transparent_black(),
            match phase {
                ButtonPhase::Hovered => neutral.foreground,
                ButtonPhase::Idle | ButtonPhase::Pressed | ButtonPhase::Disabled => {
                    neutral.foreground_muted
                }
            },
        ),
    };
    ButtonPaint {
        fill,
        border: fill,
        text,
        opacity: match phase {
            ButtonPhase::Idle | ButtonPhase::Hovered => 1.0,
            ButtonPhase::Pressed => 0.85,
            ButtonPhase::Disabled => 0.5,
        },
    }
}

/// A button, after Paseo's `button.tsx` and `control-geometry.ts`.
///
/// [Decision Log]
/// - Purpose: every delete confirmation needs the same destructive control.
/// - Constraint: existing callers still pass a primary bool; an accent's text
///   may change with a brand/theme and cannot stand in for destructive text.
/// - Alternatives: screen-local red overrides; change every caller at once.
/// - Choice: typed kind/state, the existing pill geometry and wrapping label;
///   filled kinds keep their palette on hover; ghost changes only text;
///   pressed85%, disabled50%.
/// - Why: one entry owns these values; the compatibility wrapper can retire
///   separately, without a second look for confirmations.
/// - Cost: a disabled caller attaches no handler, like the kit's outline button.
#[must_use]
pub fn button_with_kind(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    state: ButtonState,
    tokens: &Tokens,
) -> Stateful<Div> {
    let phase = match state {
        ButtonState::Enabled => ButtonPhase::Idle,
        ButtonState::Disabled => ButtonPhase::Disabled,
    };
    let paint = button_paint(kind, phase, tokens);
    let pressed = button_paint(kind, ButtonPhase::Pressed, tokens);
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(BUTTON_GAP)
        .min_h(BUTTON_HEIGHT)
        .min_w_0()
        .max_w_full()
        .px(BUTTON_HORIZONTAL)
        .rounded_full()
        .border_1()
        .border_color(paint.border)
        .bg(paint.fill)
        .text_color(paint.text)
        .opacity(paint.opacity)
        .type_(Type::Label)
        .when(state == ButtonState::Enabled, |button| {
            button
                .cursor_pointer()
                // Paseo sets no hover style on a filled button; ghost alone
                // changes its text. Press changes only opacity, so the two
                // selectors compose when the pointer is also hovering.
                .when(kind == ButtonKind::Ghost, |button| {
                    let hover = button_paint(kind, ButtonPhase::Hovered, tokens);
                    button.hover(move |style| style.text_color(hover.text))
                })
                .active(move |style| style.opacity(pressed.opacity))
        })
        .child(wrapping(label.into()))
}

/// Compatibility entry point. The lead retires this bool API after migrating
/// its callers; all drawing already delegates to the typed entry point.
#[must_use]
pub fn button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    primary: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let kind = if primary {
        ButtonKind::Primary
    } else {
        ButtonKind::Secondary
    };
    button_with_kind(id, label, kind, ButtonState::Enabled, tokens)
}

/// A button's label: on one line while it fits, wrapped and centred when
/// it does not (4 above and under each line block, so two lines do not
/// touch the pill's edge).
fn wrapping(label: SharedString) -> Div {
    div()
        .min_w_0()
        .py(px(4.0))
        .text_center()
        .whitespace_normal()
        .child(label)
}

/// Paseo's outline button (`variant="outline"`): no fill, a hairline of
/// `borderAccent`, the foreground's words; the same pill as [`button`].
/// `enabled` false fades it to half and leaves it without a pointer (the
/// screen does not attach its click).
#[must_use]
pub fn outline_button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    enabled: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    div()
        .id(id)
        .flex()
        .flex_row()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .min_h(px(32.0))
        .px(px(16.0))
        .rounded_full()
        .border_1()
        .border_color(neutral.border_accent)
        .text_color(neutral.foreground)
        .type_(Type::Label)
        .when(enabled, |button| {
            button.cursor_pointer().active(|style| style.opacity(0.85))
        })
        .when(!enabled, |button| button.opacity(0.5))
        .child(label.into())
}

/// A status dot (its word goes beside it: state is never colour alone).
#[must_use]
pub fn dot(color: Hsla) -> Div {
    div().size(px(6.0)).rounded_full().bg(color)
}

/// A column's or a section's small label, sentence case.
#[must_use]
pub fn kicker(text: impl Into<SharedString>, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Kicker)
        .text_color(tokens.text_tertiary)
        .child(text.into())
}

/// Markdown as an agent writes it: paragraphs, emphasis, code, lists.
/// The component layer's renderer, behind the kit, so a screen does not
/// depend on it; its cache is keyed by `id`, which must stay the same for
/// the same text across frames.
#[must_use]
pub fn markdown(id: impl Into<gpui::ElementId>, text: impl Into<SharedString>) -> impl IntoElement {
    gpui_component::text::TextView::markdown(id, text)
}

/// One of a few choices (a machine, an agent): a pill, filled with the
/// ink when chosen (the page's segmented choices).
#[must_use]
pub fn choice(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    chosen: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let (bg, fg, edge) = if chosen {
        (
            tokens.btn_primary_bg,
            tokens.btn_primary_fg,
            tokens.btn_primary_bg,
        )
    } else {
        (
            tokens.bg_surface,
            tokens.text_secondary,
            tokens.border_subtle,
        )
    };
    let hover = if chosen { bg } else { tokens.bg_hover };
    div()
        .id(id)
        .px(px(12.0))
        .py(px(5.0))
        .rounded(px(radius::R2))
        .border_1()
        .border_color(edge)
        .bg(bg)
        .text_color(fg)
        .type_(Type::Label)
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .child(label.into())
}

/// Vessel's mark, `size` points square: in the brand colour on the light
/// theme, in the text's colour on the dark one (the brand colour is too
/// faint on near-black: `docs/design/brand/README.md`). Beside text, pass
/// the text's size; on a first-run screen, larger.
#[must_use]
pub fn mark(size: f32, tokens: &Tokens) -> gpui::Svg {
    let color = match tokens.scheme {
        crate::Scheme::Light => gpui::rgb(crate::assets::BRAND).into(),
        crate::Scheme::Dark => tokens.text_primary,
    };
    gpui::svg()
        .path(crate::assets::Glyph::Mark.path())
        .size(px(size))
        .flex_shrink_0()
        .text_color(color)
}

/// The mark in the colour of the text beside it (the title bar).
#[must_use]
pub fn mark_mono(size: f32, color: Hsla) -> gpui::Svg {
    gpui::svg()
        .path(crate::assets::Glyph::Mark.path())
        .size(px(size))
        .flex_shrink_0()
        .text_color(color)
}

#[cfg(test)]
mod button_tests;
