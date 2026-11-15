//! One small icon chrome for pane toolbars, separate from ordinary Button sizes.
//!
//! [Decision Log]
//! - Intent: port the pinned 0.15.4 `ToolbarButton`'s small icon-button chrome.
//! - Constraint: ordinary Small28 and Medium32/44 have different geometry,
//!   glyphs and press feedback; changing them would regress their consumers.
//! - Choice: one named shared toolbar renderer with typed selection/disclosure,
//!   source20/32 frame,14 or14/18 glyph and interaction-highlight feedback.
//! - Why: every toolbar surface can reuse the same hitbox and disabled handler
//!   guard, rather than appending an icon to an empty-label ordinary button.
//! - Consequences: callers own menu state, focus and operation admission. The
//!   existing Tooltips may decorate this same element; no new listener or overlay
//!   controller, area geometry override or ordinary Button modification is added.
mod config;
#[cfg(test)]
mod tests;

pub use config::{Disclosure, Selection, ToolbarConfig, ToolbarIcon, ToolbarInk};

use crate::{ButtonState, Tooltips, tooltip::Place};
use gpui::prelude::*;
use gpui::{App, ClickEvent, Div, ElementId, SharedString, Stateful, Window, div, px};

#[derive(Clone, Copy)]
enum Phase {
    Rest,
    Hovered,
    Pressed,
}
fn fill(config: ToolbarConfig, phase: Phase, tokens: &crate::Tokens) -> gpui::Hsla {
    if config.highlighted() || !matches!(phase, Phase::Rest) {
        tokens.neutral.interaction_highlight
    } else {
        gpui::transparent_black()
    }
}

/// Small icon action with a guarded handler API and a named accessibility node.
/// It installs no keyboard listener or feature operation admission.
#[must_use]
pub struct ToolbarButton {
    element: Stateful<Div>,
    config: ToolbarConfig,
    label: SharedString,
}
impl ToolbarButton {
    /// Keep the source geometry fixed while the owner supplies its actual layout
    /// and original menu/selection state. An icon adds no empty label child.
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        icon: ToolbarIcon,
        config: ToolbarConfig,
        tokens: &crate::Tokens,
    ) -> Self {
        let label = label.into();
        let hover = fill(config, Phase::Hovered, tokens);
        let pressed = fill(config, Phase::Pressed, tokens);
        let element = div()
            .id(id)
            .role(gpui::Role::Button)
            .aria_label(label.clone())
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(config.frame())
            .p(px(crate::theme::spacing::S0))
            .rounded(px(crate::theme::radius::MD))
            .bg(fill(config, Phase::Rest, tokens))
            .opacity(config.opacity())
            .hover(move |style| style.bg(hover))
            .active(move |style| style.bg(pressed))
            .when(config.availability() == ButtonState::Enabled, |element| {
                element.cursor_pointer()
            })
            .child(icon.render(config.sizing(), tokens));
        Self {
            element,
            config,
            label,
        }
    }

    /// Disabled actions cannot acquire a normal click handler even if a caller
    /// supplies one. The owner still validates its context/ticket on dispatch.
    pub fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        if self.config.availability() == ButtonState::Enabled {
            self.element = self.element.on_click(listener);
        }
        self
    }

    /// Pinned pane toolbar profile: desktop-only, bottom/eight/300ms, using the
    /// existing screen-owned controller rather than a second toolbar overlay.
    pub fn tooltip(mut self, tips: &Tooltips, place: &Place) -> Self {
        self.element = tips.on_configured(
            self.element,
            place,
            self.label.clone(),
            crate::tooltip::Configuration::toolbar(),
            self.config.sizing(),
        );
        self
    }

    /// Keep the same source timing/offset while selecting an explicit side,
    /// such as the usage card refresh's top freshness tooltip.
    pub fn tooltip_at(
        mut self,
        tips: &Tooltips,
        place: &Place,
        side: crate::tooltip::Side,
    ) -> Self {
        self.element = tips.on_configured(
            self.element,
            place,
            self.label.clone(),
            crate::tooltip::Configuration::toolbar().side(side),
            self.config.sizing(),
        );
        self
    }
}
impl IntoElement for ToolbarButton {
    type Element = <Stateful<Div> as IntoElement>::Element;
    fn into_element(self) -> Self::Element {
        self.element.into_element()
    }
}
impl std::fmt::Debug for ToolbarButton {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ToolbarButton")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}
