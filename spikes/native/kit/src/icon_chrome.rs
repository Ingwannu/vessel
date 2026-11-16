//! Closed source profiles for the shared icon-button frame, not a second button.
//!
//! Toolbar keeps its existing state/geometry values. Sidebar Footer adds only its
//! pinned square hit area and inner round pressed layer: rounding the outer area
//! would lose corner taps. Owners retain callbacks, tooltips and admission.
use crate::toolbar_button::{ToolbarConfig, ToolbarIcon};
use crate::{ButtonState, ControlSizing, Tokens, assets::Glyph};
use gpui::prelude::*;
use gpui::{Div, ElementId, Hsla, SharedString, Stateful, div, px, svg};

const FOOTER_POINTER_FRAME: gpui::Pixels = px(28.0);
const FOOTER_COMPACT_FRAME: gpui::Pixels = px(44.0);
const FOOTER_GROUP: &str = "sidebar-footer-icon-chrome";

/// Named source profiles keep frame, glyph and feedback policy together.
#[derive(Debug, Clone, Copy)]
pub enum Profile {
    /// Existing small toolbar, including its fixed or adaptive icon mapping.
    ToolbarSmall {
        /// The owner's current availability, selection and disclosure.
        config: ToolbarConfig,
        /// One canonical vector and source ink mapping.
        icon: ToolbarIcon,
    },
    /// Sidebar footer: square28/44, glyph16/20, pressed inner round layer.
    SidebarFooter {
        /// The source icon role; no area SVG or size override.
        glyph: Glyph,
        /// Actual source layout policy, independent of the input hardware.
        sizing: ControlSizing,
    },
}

#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Rest,
    Hovered,
    Pressed,
}
pub(crate) fn toolbar_fill(config: ToolbarConfig, phase: Phase, tokens: &Tokens) -> Hsla {
    if config.highlighted() || !matches!(phase, Phase::Rest) {
        tokens.neutral.interaction_highlight
    } else {
        gpui::transparent_black()
    }
}

/// Draw one source profile. Wrappers attach their existing guarded owner handler
/// and tooltip to this same element; this function grants no effect authority.
#[must_use]
pub fn render(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    profile: Profile,
    tokens: &Tokens,
) -> Stateful<Div> {
    match profile {
        Profile::ToolbarSmall { config, icon } => toolbar_frame(
            id,
            label,
            icon.render(config.sizing(), tokens),
            config,
            tokens,
        ),
        Profile::SidebarFooter { glyph, sizing } => footer_frame(id, label, glyph, sizing, tokens),
    }
}

/// The loading toolbar can supply its existing shared ActivityIndicator while
/// retaining this exact frame. External callers cannot invent another profile.
pub(crate) fn toolbar_frame(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    content: impl IntoElement,
    config: ToolbarConfig,
    tokens: &Tokens,
) -> Stateful<Div> {
    let hover = toolbar_fill(config, Phase::Hovered, tokens);
    let pressed = toolbar_fill(config, Phase::Pressed, tokens);
    frame(id, label)
        .size(config.frame())
        .p(px(crate::theme::spacing::S0))
        .rounded(px(crate::theme::radius::MD))
        .bg(toolbar_fill(config, Phase::Rest, tokens))
        .opacity(config.opacity())
        .hover(move |style| style.bg(hover))
        .active(move |style| style.bg(pressed))
        .when(config.availability() == ButtonState::Enabled, |element| {
            element.cursor_pointer()
        })
        .child(content)
}
fn frame(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(label)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
}
fn footer_frame(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    glyph: Glyph,
    sizing: ControlSizing,
    tokens: &Tokens,
) -> Stateful<Div> {
    let id = id.into();
    let (edge, _) = footer_geometry(sizing);
    frame(id.clone(), label)
        .relative()
        .group(FOOTER_GROUP)
        .size(edge)
        .p(px(crate::theme::spacing::S1))
        .cursor_pointer()
        .child(footer_highlight(id, tokens))
        .child(footer_icon(glyph, sizing, tokens))
}
fn footer_geometry(sizing: ControlSizing) -> (gpui::Pixels, gpui::Pixels) {
    match sizing {
        ControlSizing::Pointer => (FOOTER_POINTER_FRAME, px(crate::theme::icon::MD)),
        ControlSizing::Touch => (FOOTER_COMPACT_FRAME, px(crate::theme::icon::LG)),
    }
}
fn footer_ink(phase: Phase, tokens: &Tokens) -> Hsla {
    match phase {
        Phase::Hovered => tokens.neutral.foreground,
        Phase::Rest | Phase::Pressed => tokens.neutral.foreground_muted,
    }
}
fn footer_highlight(id: ElementId, tokens: &Tokens) -> Stateful<Div> {
    let pressed_fill = tokens.neutral.interaction_highlight;
    // This decorative child registers no handler and does not occlude the
    // square parent hitbox. The nearest parent group owns pressed state.
    div()
        .id(ElementId::NamedChild(
            std::sync::Arc::new(id),
            "pressed-layer".into(),
        ))
        .absolute()
        .inset_0()
        .rounded_full()
        .bg(gpui::transparent_black())
        .group_active(FOOTER_GROUP, move |style| style.bg(pressed_fill))
}
fn footer_icon(glyph: Glyph, sizing: ControlSizing, tokens: &Tokens) -> gpui::Svg {
    let (_, icon) = footer_geometry(sizing);
    let hover_ink = footer_ink(Phase::Hovered, tokens);
    svg()
        .path(glyph.path())
        .size(icon)
        .flex_none()
        .text_color(footer_ink(Phase::Rest, tokens))
        .group_hover(FOOTER_GROUP, move |style| style.text_color(hover_ink))
}

#[cfg(test)]
mod tests;
