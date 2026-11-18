//! Closed source profiles for the shared icon-button frame, not a second button.
//!
//! Toolbar keeps its existing state/geometry values. Sidebar Footer adds only its
//! pinned square hit area and inner round pressed layer: rounding the outer area
//! would lose corner taps. Owners retain callbacks, tooltips and admission.
//!
//! [Decision Log] Viewport profile (2026-11-16)
//! - Intent: Paseo's zoomable-viewport toolbar (toolbar.tsx:75-136) is the same
//!   small chrome (`iconButtonChromeStyle` small, 20/32 frame, 14/18 glyph) with a
//!   surface2 plate, a muted glyph that turns foreground under the pointer, and
//!   opacity 0 with no pointer events while its viewport is not hovered,
//!   focused or touched.
//! - Constraint: the Toolbar profile's rest fill is transparent and its ink is
//!   fixed; an area may not restyle the frame (no second button style).
//! - Alternatives: a lightbox-local button (a second style, rejected by the
//!   coordinator's return); a fill/ink knob on `ToolbarConfig` (every toolbar
//!   caller would carry options only one surface uses).
//! - Choice: one closed `Viewport` profile beside the two others, reusing the
//!   toolbar geometry, highlight and disabled opacity; `Presence` is the owner's
//!   shown/hidden fact, supplied like `ControlSizing`.
//! - Consequences: the owner decides presence (hover, focus, touch) and keeps
//!   its callbacks; a hidden frame keeps its layout and accessible name, as
//!   Paseo's opacity 0 does, and `ToolbarButton::viewport` attaches no handler.
use crate::toolbar_button::{ToolbarConfig, ToolbarIcon, ToolbarInk};
use crate::{ButtonState, ControlSizing, Tokens, assets::Glyph};
use gpui::prelude::*;
use gpui::{Div, ElementId, Hsla, SharedString, Stateful, div, px, svg};

const FOOTER_POINTER_FRAME: gpui::Pixels = px(28.0);
const FOOTER_COMPACT_FRAME: gpui::Pixels = px(44.0);
const FOOTER_GROUP: &str = "sidebar-footer-icon-chrome";
const VIEWPORT_GROUP: &str = "viewport-icon-chrome";
/// Paseo's `buttonHidden`: an enabled frame stays laid out and named, unseen.
const HIDDEN_OPACITY: f32 = 0.0;

/// Whether a viewport's controls are shown: the owner's hover, focus or touch
/// fact, decided once at the area edge like [`ControlSizing`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// Visible and able to take the pointer.
    Shown,
    /// Laid out and named; unseen unless disabled (Paseo's style order keeps
    /// a disabled action at 0.5); no hover feedback, no handler.
    Hidden,
}

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
    /// A zoomable viewport's floating action: the small toolbar frame on a
    /// surface2 plate, a muted adaptive14/18 glyph turning foreground on hover.
    Viewport {
        /// The owner's current availability and actual layout.
        config: ToolbarConfig,
        /// The source icon role.
        glyph: Glyph,
        /// The owner's shown/hidden fact.
        presence: Presence,
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
        Profile::Viewport {
            config,
            glyph,
            presence,
        } => viewport_frame(id, label, glyph, config, presence, tokens),
    }
}

/// The loading toolbar can supply its existing shared `ActivityIndicator` while
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
        // Paseo's header island: a round hitbox and highlight.
        .when(config.island(), Styled::rounded_full)
        .bg(toolbar_fill(config, Phase::Rest, tokens))
        .opacity(config.opacity())
        // Paseo's disabled `Pressable` has no hovered or pressed state, as in
        // the viewport profile (`viewport_responds`): only an enabled action
        // answers the pointer (TRACKER 776 item 7).
        .when(config.availability() == ButtonState::Enabled, |element| {
            element
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .active(move |style| style.bg(pressed))
        })
        .child(content)
}
/// Paseo's `highlighted` replaces the plate (`interactionHighlight` over the
/// viewport), so hover and press read exactly as on every other toolbar.
pub(crate) fn viewport_fill(config: ToolbarConfig, phase: Phase, tokens: &Tokens) -> Hsla {
    if config.highlighted() || !matches!(phase, Phase::Rest) {
        tokens.neutral.interaction_highlight
    } else {
        tokens.neutral.surface2
    }
}
/// Paseo's disabled `Pressable` has no hovered or pressed state
/// (react-native-web `Pressable`: `useHover` and `usePressEvents` take
/// `disabled`), and a hidden one takes no pointer: only an enabled, shown
/// action answers the pointer with highlight and foreground ink.
pub(crate) fn viewport_responds(config: ToolbarConfig, presence: Presence) -> bool {
    presence == Presence::Shown && config.availability() == ButtonState::Enabled
}
pub(crate) fn viewport_frame(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    glyph: Glyph,
    config: ToolbarConfig,
    presence: Presence,
    tokens: &Tokens,
) -> Stateful<Div> {
    let hover_ink = tokens.neutral.foreground;
    let icon = ToolbarIcon::new(glyph, ToolbarInk::Muted)
        .adaptive()
        .render(config.sizing(), tokens);
    // Paseo's style order (icon-button-chrome.ts:79-82): the caller's hidden
    // style, then highlighted, then disabled last, so a hidden disabled
    // action still shows at 0.5 (a faint Scan at fit) and takes no pointer.
    let opacity = if presence == Presence::Hidden && config.availability() == ButtonState::Enabled {
        HIDDEN_OPACITY
    } else {
        config.opacity()
    };
    let element = frame(id, label)
        .group(VIEWPORT_GROUP)
        .size(config.frame())
        .p(px(crate::theme::spacing::S0))
        .rounded(px(crate::theme::radius::MD))
        .bg(viewport_fill(config, Phase::Rest, tokens))
        .opacity(opacity);
    if !viewport_responds(config, presence) {
        return element.child(icon);
    }
    let hover = viewport_fill(config, Phase::Hovered, tokens);
    let pressed = viewport_fill(config, Phase::Pressed, tokens);
    element
        .hover(move |style| style.bg(hover))
        .active(move |style| style.bg(pressed))
        .cursor_pointer()
        .child(icon.group_hover(VIEWPORT_GROUP, move |style| style.text_color(hover_ink)))
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
