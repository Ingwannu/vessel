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
use gpui::{App, ClickEvent, Div, ElementId, SharedString, Stateful, Window};

/// Small icon action with a guarded handler API and a named accessibility node.
/// It installs no keyboard listener or feature operation admission.
#[must_use]
pub struct ToolbarButton {
    element: Stateful<Div>,
    config: ToolbarConfig,
    label: SharedString,
    shortcut: std::rc::Rc<[SharedString]>,
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
        Self::with_content(
            id,
            label,
            icon.render(config.sizing(), tokens),
            config,
            tokens,
        )
    }

    /// The pinned usage refresh's numeric14/18 `ActivityIndicator` uses the SAME
    /// shared masks/750ms motion. Loading chrome is disabled and acquires no click
    /// handler; the owner still retains original request/context admission.
    pub fn loading(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        ink: ToolbarInk,
        config: ToolbarConfig,
        tokens: &crate::Tokens,
    ) -> Self {
        let id = id.into();
        let points = match config.sizing() {
            crate::ControlSizing::Pointer => crate::loading::Points::FOURTEEN,
            crate::ControlSizing::Touch => crate::loading::Points::EIGHTEEN,
        };
        let content = crate::loading::activity_indicator(
            ElementId::NamedChild(std::sync::Arc::new(id.clone()), "activity".into()),
            crate::loading::Size::Numeric(points),
            ink.colour(tokens),
        );
        Self::with_content(id, label, content, config.disabled(), tokens)
    }

    /// A zoomable viewport's floating action (`icon_chrome::Profile::Viewport`).
    /// A hidden action is inert as a disabled one is: no handler can attach,
    /// as Paseo's hidden buttons take no pointer events.
    pub fn viewport(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        glyph: crate::assets::Glyph,
        config: ToolbarConfig,
        presence: crate::icon_chrome::Presence,
        tokens: &crate::Tokens,
    ) -> Self {
        let label = label.into();
        let element =
            crate::icon_chrome::viewport_frame(id, label.clone(), glyph, config, presence, tokens);
        let config = match presence {
            crate::icon_chrome::Presence::Shown => config,
            crate::icon_chrome::Presence::Hidden => config.disabled(),
        };
        Self {
            element,
            config,
            label,
            shortcut: std::rc::Rc::from([]),
        }
    }

    fn with_content(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        content: impl IntoElement,
        config: ToolbarConfig,
        tokens: &crate::Tokens,
    ) -> Self {
        let label = label.into();
        let element = crate::icon_chrome::toolbar_frame(id, label.clone(), content, config, tokens);
        Self {
            element,
            config,
            label,
            shortcut: std::rc::Rc::from([]),
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

    /// The bound shortcut's chords, shown after the words in this button's
    /// tooltip (Paseo's `ToolbarButton shortcut={keys}`); empty shows none.
    #[must_use]
    pub fn shortcut(mut self, chords: Vec<SharedString>) -> Self {
        self.shortcut = std::rc::Rc::from(chords);
        self
    }

    /// Pinned pane toolbar profile: desktop-only, bottom/eight/300ms, using the
    /// existing screen-owned controller rather than a second toolbar overlay.
    pub fn tooltip(self, tips: &Tooltips, place: &Place) -> Self {
        let label = self.label.clone();
        self.tooltip_text(tips, place, label, crate::tooltip::Side::Bottom)
    }

    /// Keep the same source timing/offset while selecting an explicit side,
    /// such as the usage card refresh's top freshness tooltip.
    pub fn tooltip_at(self, tips: &Tooltips, place: &Place, side: crate::tooltip::Side) -> Self {
        let label = self.label.clone();
        self.tooltip_text(tips, place, label, side)
    }

    /// Source tooltip words may describe freshness/status while the accessibility
    /// label remains the action. The owner supplies already-resolved display text.
    pub fn tooltip_text(
        mut self,
        tips: &Tooltips,
        place: &Place,
        text: impl Into<SharedString>,
        side: crate::tooltip::Side,
    ) -> Self {
        self.element = tips.on_configured_keyed(
            self.element,
            place,
            text,
            std::rc::Rc::clone(&self.shortcut),
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
