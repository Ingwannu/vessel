//! Pointer focus on the padding of settings inputs, not only their text.
//!
//! [Decision Log]
//! - Purpose: the reference `TextInput`'s insets belong to the interactive field.
//! - Constraint: the existing engine owns caret placement/selection and IME;
//!   an engine-consumed press must not run a second handler in its outer shell.
//! - Alternatives: make the padding a dead region, or add an editing engine.
//! - Choice: forward an unconsumed inset press through the engine's public
//!   focus/presentation APIs, including its touch-keyboard eligibility.
//! - Costs: only the reserved number/rename shells use this bridge; original
//!   line/composer/multiline geometry and handlers remain unchanged.
use gpui::prelude::*;
use gpui::{Div, Entity, MouseButton, div};
use gpui_base::input::InputState;

pub(super) fn frame(state: &Entity<InputState>) -> Div {
    let state = state.clone();
    div()
        .cursor(gpui::CursorStyle::IBeam)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            if window.default_prevented() {
                return;
            }
            let input = state.read(cx).presentation();
            if input.is_disabled() {
                return;
            }
            if gpui_base::GlobalState::is_touch_press(cx) && input.is_editable() {
                window.request_virtual_keyboard();
            }
            input.focus_handle().focus(window, cx);
        })
}
