//! Configuration and input guard around the same shared button drawing. Loading
//! replaces its leading with the source Small20 indicator and denies activation.
use super::{ButtonGeometry, ButtonKind, ButtonState};
use crate::control_sizing::ControlSizing;
use gpui::prelude::*;
use gpui::{App, ClickEvent, Div, Stateful, StyleRefinement, Window, px};

/// Source semantic size, separate from the pointer/touch layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    /// Tight controls, including catalog Add:28/px12 in both layouts.
    Small,
    /// Default:32/px16 pointer,44/px24 touch.
    Medium,
}
const SMALL_HEIGHT: gpui::Pixels = px(28.0);
const SMALL_HORIZONTAL: gpui::Pixels = px(12.0);
const SMALL_ICON: gpui::Pixels = px(12.0);

impl ButtonSize {
    pub(super) fn geometry(self, sizing: ControlSizing) -> ButtonGeometry {
        match self {
            Self::Small => ButtonGeometry {
                height: SMALL_HEIGHT,
                horizontal: SMALL_HORIZONTAL,
            },
            Self::Medium => super::button_geometry(sizing),
        }
    }
    pub(super) const fn icon(self) -> gpui::Pixels {
        match self {
            Self::Small => SMALL_ICON,
            Self::Medium => super::BUTTON_ICON,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Activity {
    Idle,
    Loading,
}
/// Every input to one button's source geometry and feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonConfig {
    pub(super) kind: ButtonKind,
    pub(super) state: ButtonState,
    pub(super) sizing: ControlSizing,
    pub(super) size: ButtonSize,
    pub(super) activity: Activity,
}
impl ButtonConfig {
    /// Default MD; the caller supplies the actual window's layout once.
    #[must_use]
    pub const fn new(kind: ButtonKind, state: ButtonState, sizing: ControlSizing) -> Self {
        Self {
            kind,
            state,
            sizing,
            size: ButtonSize::Medium,
            activity: Activity::Idle,
        }
    }
    /// Explicit source size, with no area scaling or vertical-padding override.
    #[must_use]
    pub const fn with_size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }
    /// Pending replaces the leading glyph and disables the whole action.
    #[must_use]
    pub const fn loading(mut self) -> Self {
        self.activity = Activity::Loading;
        self
    }
    /// Effective input state, including the source disabled-or-loading rule.
    #[must_use]
    pub const fn availability(self) -> ButtonState {
        match (self.state, self.activity) {
            (ButtonState::Enabled, Activity::Idle) => ButtonState::Enabled,
            (ButtonState::Disabled, _) | (_, Activity::Loading) => ButtonState::Disabled,
        }
    }
}
/// A content/loading button whose normal handler API cannot activate while
/// disabled or pending. Legacy entries keep their Stateful<Div> return type.
#[must_use]
pub struct Button {
    element: Stateful<Div>,
    state: ButtonState,
}
impl Button {
    pub(super) fn new(element: Stateful<Div>, state: ButtonState) -> Self {
        Self { element, state }
    }
    /// Name an icon action through the same button's accessibility node.
    /// This forwards role/label; it does not create a keyboard binding.
    pub fn aria_label(mut self, label: impl Into<gpui::SharedString>) -> Self {
        self.element = self.element.role(gpui::Role::Button).aria_label(label);
        self
    }
    /// Attach only when enabled, even if a consumer supplies a callback while
    /// pending. View must still guard its own state transition at dispatch.
    pub fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        if self.state == ButtonState::Enabled {
            self.element = self.element.on_click(listener);
        }
        self
    }
}
impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        self.element.style()
    }
}
impl IntoElement for Button {
    type Element = <Stateful<Div> as IntoElement>::Element;
    fn into_element(self) -> Self::Element {
        self.element.into_element()
    }
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}
