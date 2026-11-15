//! One filled/ghost button renderer. Label-only entries keep their signatures
//! and delegate to typed content; an icon-only button has no empty text slot.
//!
//! [Decision Log]
//! - Purpose: shared settings reorder and form actions need the same button.
//! - Constraints: existing callers/paint/default32-pointer geometry stay intact.
//! - Choice: `Label`/`Icon`/`IconAndLabel` plus a typed automatic-or-muted icon.
//!   The source's explicit muted `ReactElement` stays muted even on Ghost hover;
//!   automatic icons use the button ink. Both put ink on the SVG itself.
//! - Alternative: an area arrow div, empty text children, or a toolbar pill.
//! - Why and cost: one renderer/interaction policy; callers still attach no
//!   handler to Disabled, as required by the existing button contract.
use super::TypeExt as _;
use crate::{control_sizing::ControlSizing, theme::Tokens};
use gpui::prelude::*;
use gpui::{Div, Hsla, SharedString, Stateful, div, px};

/// Button meanings from Paseo (`default`, `secondary`, `destructive`,
/// `ghost` and `outline`). The legacy outline entry delegates here too.
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
    /// Transparent fill and the reference `borderAccent`, for catalog Add.
    Outline,
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

mod content;
mod control;
mod design;
pub use content::{ButtonContent, ButtonIcon};
#[cfg(test)]
use content::{icon_element, icon_ink};
use control::Activity;
pub use control::{Button, ButtonConfig, ButtonSize};
pub use design::{ButtonDesign, CircleConfig, CircleTone};

const BUTTON_GROUP: &str = "kit-button";
const BUTTON_ICON: gpui::Pixels = px(crate::theme::icon::SM);

const BUTTON_HEIGHT: gpui::Pixels = px(32.0);
const BUTTON_HORIZONTAL: gpui::Pixels = px(16.0);
const BUTTON_GAP: gpui::Pixels = px(8.0);
const LABEL_VERTICAL: gpui::Pixels = px(crate::theme::spacing::S1);

#[derive(Debug, Clone, Copy, PartialEq)]
struct ButtonGeometry {
    height: gpui::Pixels,
    horizontal: gpui::Pixels,
}
const fn button_geometry(sizing: ControlSizing) -> ButtonGeometry {
    match sizing {
        ControlSizing::Pointer => ButtonGeometry {
            height: BUTTON_HEIGHT,
            horizontal: BUTTON_HORIZONTAL,
        },
        ControlSizing::Touch => ButtonGeometry {
            height: px(crate::control_sizing::TOUCH_TARGET),
            horizontal: px(crate::theme::spacing::S6),
        },
    }
}

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
        ButtonKind::Outline => (gpui::transparent_black(), neutral.foreground),
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
        border: match kind {
            ButtonKind::Outline => neutral.border_accent,
            ButtonKind::Primary
            | ButtonKind::Secondary
            | ButtonKind::Destructive
            | ButtonKind::Ghost => fill,
        },
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
    button_with_kind_at(id, label, kind, state, ControlSizing::Pointer, tokens)
}

/// The same shared button at the source's explicit pointer/touch geometry.
/// Legacy entries remain Pointer until their owners migrate their callers.
/// They receive neither Window nor App, so a multi-window layout cannot be
/// discovered safely here; no global or guessed compact state is consulted.
#[must_use]
pub fn button_with_kind_at(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    state: ButtonState,
    sizing: ControlSizing,
    tokens: &Tokens,
) -> Stateful<Div> {
    render(
        id,
        ButtonContent::Label(label.into()),
        ButtonConfig::new(kind, state, sizing).into(),
        Presentation::PublishedFilled,
        tokens,
    )
}

/// Typed content through the same button renderer; an icon-only MD button is a
///32/44 square at Pointer/Touch, with its14-point source vector and no label.
#[must_use]
pub fn button_with_content(
    id: impl Into<gpui::ElementId>,
    content: ButtonContent,
    kind: ButtonKind,
    state: ButtonState,
    sizing: ControlSizing,
    tokens: &Tokens,
) -> Stateful<Div> {
    render(
        id,
        content,
        ButtonConfig::new(kind, state, sizing).into(),
        Presentation::Standard,
        tokens,
    )
}

/// Loading/size-aware content with a guarded handler entry. Catalog Add uses
/// Small+Outline; loading is the source Small20 `ActivityIndicator` before text.
pub fn button_control(
    id: impl Into<gpui::ElementId>,
    content: ButtonContent,
    config: ButtonConfig,
    tokens: &Tokens,
) -> Button {
    button_with_design(id, content, config.into(), tokens)
}
/// A complete regular or source-circle configuration through the same body.
/// The compatibility `button_control` signature stays unchanged.
pub fn button_with_design(
    id: impl Into<gpui::ElementId>,
    content: ButtonContent,
    config: ButtonDesign,
    tokens: &Tokens,
) -> Button {
    Button::new(
        render(id, content, config, Presentation::Standard, tokens),
        config.availability(),
    )
}
// The published outline API passes a caller-selected tone and direct text
// child. Keep that contract (including its actual child bounds) inside the
// same frame/content renderer, rather than replacing the lead's Remove action.
#[derive(Clone, Copy)]
enum Presentation {
    Standard,
    PublishedFilled,
    PublishedOutline(Hsla),
}
fn frame(config: ButtonDesign, presentation: Presentation, tokens: &Tokens) -> Div {
    let phase = match config.availability() {
        ButtonState::Enabled => ButtonPhase::Idle,
        ButtonState::Disabled => ButtonPhase::Disabled,
    };
    let paint = config.paint(phase, tokens);
    let geometry = config.geometry();
    let body = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(BUTTON_GAP)
        .min_h(geometry.height)
        .px(geometry.horizontal)
        .rounded_full()
        .border_1()
        .border_color(paint.border)
        .opacity(paint.opacity)
        .type_(config.type_step());
    match presentation {
        Presentation::Standard | Presentation::PublishedFilled => body
            .when(!config.is_circle(), |body| body.min_w_0().max_w_full())
            .bg(paint.fill)
            .text_color(paint.text)
            .font_weight(gpui::FontWeight::MEDIUM),
        Presentation::PublishedOutline(tone) => body.flex_shrink_0().text_color(tone),
    }
}
fn body(
    content: ButtonContent,
    config: ButtonDesign,
    presentation: Presentation,
    tokens: &Tokens,
) -> Div {
    let content = content.parts();
    let geometry = config.geometry();
    let mut body = frame(config, presentation, tokens);
    if config.is_circle() {
        body = body.size(geometry.height).px(px(0.0)).flex_none();
    } else if content.label.is_none() {
        body = body.w(geometry.height).px(px(0.0)).flex_none();
    }
    body = match config.activity() {
        Activity::Loading => body.child(crate::loading::activity_indicator(
            "button-loading",
            crate::loading::Size::Small,
            config.paint(ButtonPhase::Idle, tokens).text,
        )),
        Activity::Idle => match content.leading {
            Some(icon) => {
                let element = config.icon(icon, tokens);
                body.child(match presentation {
                    Presentation::Standard | Presentation::PublishedFilled => element,
                    Presentation::PublishedOutline(tone) => element.text_color(tone),
                })
            }
            None => body,
        },
    };
    match (content.label, presentation) {
        (Some(label), Presentation::Standard) => body.child(wrapping(label)),
        (Some(label), Presentation::PublishedFilled) => {
            body.child(wrapping(label).py(LABEL_VERTICAL))
        }
        (Some(label), Presentation::PublishedOutline(_)) => body.child(label),
        (None, _) => body,
    }
}
fn render(
    id: impl Into<gpui::ElementId>,
    content: ButtonContent,
    config: ButtonDesign,
    presentation: Presentation,
    tokens: &Tokens,
) -> Stateful<Div> {
    let state = config.availability();
    let hover = config.hover_ink(tokens);
    let pressed = config.pressed_opacity(tokens);
    body(content, config, presentation, tokens)
        .id(id)
        .group(BUTTON_GROUP)
        .when(state == ButtonState::Enabled, |button| {
            button
                .cursor_pointer()
                // Circle's source has no hover or pressed override.
                .when_some(hover, |button, ink| button.hover(move |style| style.text_color(ink)))
                .when_some(pressed, |button, opacity| button.active(move |style| style.opacity(opacity)))
        })
}
fn outline_config(enabled: bool) -> ButtonConfig {
    ButtonConfig::new(
        ButtonKind::Outline,
        if enabled {
            ButtonState::Enabled
        } else {
            ButtonState::Disabled
        },
        ControlSizing::Pointer,
    )
}
pub(super) fn outline_pressable(
    body: Div,
    id: impl Into<gpui::ElementId>,
    enabled: bool,
) -> Stateful<Div> {
    body.id(id).when(enabled, |button| {
        button.cursor_pointer().active(|style| style.opacity(0.85))
    })
}
pub(super) fn outline_label(
    id: impl Into<gpui::ElementId>,
    label: SharedString,
    enabled: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    outline_pressable(
        body(
            ButtonContent::Label(label),
            outline_config(enabled).into(),
            Presentation::PublishedOutline(tokens.neutral.foreground),
            tokens,
        ),
        id,
        enabled,
    )
}
pub(super) fn outline_glyph_body(
    glyph: crate::assets::Glyph,
    label: SharedString,
    tone: Hsla,
    enabled: bool,
    tokens: &Tokens,
) -> Div {
    body(
        ButtonContent::IconAndLabel {
            icon: ButtonIcon::new(glyph),
            label,
        },
        outline_config(enabled).into(),
        Presentation::PublishedOutline(tone),
        tokens,
    )
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

/// Source text wraps without additional vertical padding. Published filled
/// entries explicitly retain their older four-point inset in the shared body.
fn wrapping(label: SharedString) -> Div {
    div()
        .min_w_0()
        .text_center()
        .whitespace_normal()
        .child(label)
}

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;

#[cfg(test)]
mod input_tests;
#[cfg(test)]
mod layout_tests;

#[cfg(test)]
mod circle_tests;
