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

/// A button: the primary one is the ink with white text, the secondary
/// one a light well (DESIGN.md section 6, `--btn-*`).
#[must_use]
pub fn button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    primary: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let (bg, fg, hover) = if primary {
        (
            tokens.btn_primary_bg,
            tokens.btn_primary_fg,
            tokens.btn_primary_bg,
        )
    } else {
        (
            tokens.btn_secondary_bg,
            tokens.text_primary,
            tokens.btn_secondary_bg_hover,
        )
    };
    div()
        .id(id)
        .px(px(12.0))
        .py(px(6.0))
        .rounded(px(radius::R2))
        .bg(bg)
        .text_color(fg)
        .type_(Type::Label)
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
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
