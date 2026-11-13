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
