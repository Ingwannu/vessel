//! Typed children and own SVG ink, shared by every button variant/size.
use super::{BUTTON_GROUP, ButtonKind, ButtonPhase, ButtonSize, ButtonState, button_paint};
use crate::{assets::Glyph, theme::Tokens};
use gpui::prelude::*;
use gpui::{Hsla, SharedString, svg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IconTone {
    Button,
    Muted,
}
/// A shared vector with the source's automatic or explicitly muted ink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonIcon {
    glyph: Glyph,
    tone: IconTone,
}
impl ButtonIcon {
    /// Inherit this button variant's own text/hover ink.
    #[must_use]
    pub const fn new(glyph: Glyph) -> Self {
        Self {
            glyph,
            tone: IconTone::Button,
        }
    }
    /// Explicit muted element, as settings `SidebarNavRow`'s reorder arrows.
    #[must_use]
    pub const fn muted(glyph: Glyph) -> Self {
        Self {
            glyph,
            tone: IconTone::Muted,
        }
    }
}
/// Button children, with no empty-label workaround for icon-only controls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ButtonContent {
    /// Existing text-only behavior, including its wrapping policy.
    Label(SharedString),
    /// Square icon-only control.
    Icon(ButtonIcon),
    /// Leading vector and the ordinary label, in that order.
    IconAndLabel {
        /// Leading shared vector.
        icon: ButtonIcon,
        /// Ordinary wrapping label.
        label: SharedString,
    },
}
pub(super) fn icon_ink(
    icon: ButtonIcon,
    kind: ButtonKind,
    phase: ButtonPhase,
    tokens: &Tokens,
) -> Hsla {
    match icon.tone {
        IconTone::Button => button_paint(kind, phase, tokens).text,
        IconTone::Muted => tokens.neutral.foreground_muted,
    }
}
pub(super) fn icon_element(
    icon: ButtonIcon,
    kind: ButtonKind,
    state: ButtonState,
    size: ButtonSize,
    tokens: &Tokens,
) -> gpui::Svg {
    let ink = icon_ink(icon, kind, ButtonPhase::Idle, tokens);
    let hover = icon_ink(icon, kind, ButtonPhase::Hovered, tokens);
    let hover = (state == ButtonState::Enabled
        && kind == ButtonKind::Ghost
        && icon.tone == IconTone::Button)
        .then_some(hover);
    draw_icon(icon, size.icon(), ink, hover)
}

pub(super) fn circle_icon(icon: ButtonIcon, automatic: Hsla, tokens: &Tokens) -> gpui::Svg {
    let ink = match icon.tone {
        IconTone::Button => automatic,
        IconTone::Muted => tokens.neutral.foreground_muted,
    };
    draw_icon(icon, super::design::CIRCLE_ICON, ink, None)
}
fn draw_icon(icon: ButtonIcon, size: gpui::Pixels, ink: Hsla, hover: Option<Hsla>) -> gpui::Svg {
    svg()
        .path(icon.glyph.path())
        .size(size)
        .flex_none()
        .text_color(ink)
        .when_some(hover, |element, ink| {
            element.group_hover(BUTTON_GROUP, move |style| style.text_color(ink))
        })
}

pub(super) struct Parts {
    pub leading: Option<ButtonIcon>,
    pub label: Option<SharedString>,
}
impl ButtonContent {
    /// Its words, which also name it for assistive technology; none for an
    /// icon-only control.
    pub(super) fn name(&self) -> Option<SharedString> {
        match self {
            Self::Label(label) | Self::IconAndLabel { label, .. } => Some(label.clone()),
            Self::Icon(_) => None,
        }
    }
    pub(super) fn parts(self) -> Parts {
        match self {
            Self::Label(label) => Parts {
                leading: None,
                label: Some(label),
            },
            Self::Icon(icon) => Parts {
                leading: Some(icon),
                label: None,
            },
            Self::IconAndLabel { icon, label } => Parts {
                leading: Some(icon),
                label: Some(label),
            },
        }
    }
}
