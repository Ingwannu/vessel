//! Focus across a settings field's inset, shared by its different profiles.
//! No paint, geometry, input state or event subscription is added here.

use super::TextField;
use gpui::prelude::*;
use gpui::{Div, MouseButton, div};

/// A padding click focuses the existing editor without reselecting a caret
/// the person placed. Profiles supply their own width, chrome and bare input.
pub(crate) fn settings_frame(field: &TextField) -> Div {
    let field = field.clone();
    div()
        .cursor_text()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            field.focus(window, cx);
        })
}
