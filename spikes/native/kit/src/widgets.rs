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
}

impl<E: Styled> TypeExt for E {}

/// A button, as Paseo's (`components/ui/button.tsx`, size `md`): a pill
/// at least 32 tall, 16 across, 14 at weight 500, a clear hairline; the
/// primary one the accent with white words, the secondary `surface3` with
/// the foreground's; pressed at 85%. No hover fill (Paseo sets none).
#[must_use]
pub fn button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    primary: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let (bg, fg) = if primary {
        (neutral.accent, neutral.accent_foreground)
    } else {
        (neutral.surface3, neutral.foreground)
    };
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .min_h(px(32.0))
        .px(px(16.0))
        .rounded_full()
        .border_1()
        .border_color(gpui::transparent_black())
        .bg(bg)
        .text_color(fg)
        .type_(Type::Label)
        .cursor_pointer()
        .active(|style| style.opacity(0.85))
        .child(label.into())
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
