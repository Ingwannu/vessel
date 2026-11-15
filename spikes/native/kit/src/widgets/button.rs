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
use crate::{control_sizing::ControlSizing, fonts::Type, theme::Tokens};
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
use content::icon_element;
#[cfg(test)]
use content::icon_ink;
pub use content::{ButtonContent, ButtonIcon};
use control::Activity;
pub use control::{Button, ButtonConfig, ButtonSize};

const BUTTON_GROUP: &str = "kit-button";
const BUTTON_ICON: gpui::Pixels = px(crate::theme::icon::SM);

const BUTTON_HEIGHT: gpui::Pixels = px(32.0);
const BUTTON_HORIZONTAL: gpui::Pixels = px(16.0);
const BUTTON_GAP: gpui::Pixels = px(8.0);

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
            _ => fill,
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
    button_with_content(
        id,
        ButtonContent::Label(label.into()),
        kind,
        state,
        sizing,
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
    render(id, content, ButtonConfig::new(kind, state, sizing), tokens)
}

/// Loading/size-aware content with a guarded handler entry. Catalog Add uses
/// Small+Outline; loading is the source Small20 ActivityIndicator before text.
#[must_use]
pub fn button_control(
    id: impl Into<gpui::ElementId>,
    content: ButtonContent,
    config: ButtonConfig,
    tokens: &Tokens,
) -> Button {
    Button::new(render(id, content, config, tokens), config.availability())
}
fn render(
    id: impl Into<gpui::ElementId>,
    content: ButtonContent,
    config: ButtonConfig,
    tokens: &Tokens,
) -> Stateful<Div> {
    let ButtonConfig {
        kind,
        sizing,
        size,
        activity,
        ..
    } = config;
    let state = config.availability();
    let geometry = size.geometry(sizing);
    let phase = match state {
        ButtonState::Enabled => ButtonPhase::Idle,
        ButtonState::Disabled => ButtonPhase::Disabled,
    };
    let paint = button_paint(kind, phase, tokens);
    let pressed = button_paint(kind, ButtonPhase::Pressed, tokens);
    let button = div()
        .id(id)
        .group(BUTTON_GROUP)
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(BUTTON_GAP)
        .min_h(geometry.height)
        .min_w_0()
        .max_w_full()
        .px(geometry.horizontal)
        .rounded_full()
        .border_1()
        .border_color(paint.border)
        .bg(paint.fill)
        .text_color(paint.text)
        .opacity(paint.opacity)
        .type_(match size {
            ButtonSize::Small => Type::Meta,
            ButtonSize::Medium => Type::Label,
        })
        .font_weight(gpui::FontWeight::MEDIUM)
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
        });
    let content = content.parts();
    let mut button = if content.label.is_none() {
        button.w(geometry.height).px(px(0.0)).flex_none()
    } else {
        button
    };
    button = match activity {
        Activity::Loading => button.child(crate::loading::activity_indicator(
            "button-loading",
            crate::loading::Size::Small,
            paint.text,
        )),
        Activity::Idle => match content.leading {
            Some(icon) => button.child(icon_element(icon, kind, state, size, tokens)),
            None => button,
        },
    };
    button.children(content.label.map(wrapping))
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

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;

#[cfg(test)]
mod input_tests;
