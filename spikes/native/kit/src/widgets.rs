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

mod button;
pub use button::{
    Button, ButtonConfig, ButtonContent, ButtonDesign, ButtonIcon, ButtonKind, ButtonSize,
    ButtonState, CircleConfig, CircleTone, button, button_control, button_with_content,
    button_with_design, button_with_kind, button_with_kind_at,
};

/// Paseo's outline button (`variant="outline"`): no fill, a hairline of
/// `borderAccent`, the foreground's words; the same pill as [`button()`].
/// `enabled` false fades it to half and leaves it without a pointer (the
/// screen does not attach its click).
#[must_use]
pub fn outline_button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    enabled: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    button::outline_label(id, label.into(), enabled, tokens)
}

/// Outline pill with a leading14-point vector and words in the caller's tone.
/// Preserves the published Remove-machine API and its immediate-child geometry.
#[must_use]
pub fn outline_button_with_glyph(
    id: impl Into<gpui::ElementId>,
    glyph: crate::assets::Glyph,
    label: impl Into<SharedString>,
    tone: Hsla,
    enabled: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let label = label.into();
    button::outline_pressable(
        glyph_body(glyph, label.clone(), tone, enabled, tokens),
        id,
        label,
        enabled,
    )
}

fn glyph_body(
    glyph: crate::assets::Glyph,
    label: impl Into<SharedString>,
    tone: Hsla,
    enabled: bool,
    tokens: &Tokens,
) -> Div {
    button::outline_glyph_body(glyph, label.into(), tone, enabled, tokens)
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
