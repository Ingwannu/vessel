//! Whole-value selection over the existing line and multiline engines.
//! No editing state, event subscription or editability definition lives here.

use super::{Kind, TextField};
use gpui::{App, Window};

impl TextField {
    /// Select the existing value when a form opens. The existing engine owns
    /// UTF-8/UTF-16 selection and the undo transaction boundary.
    pub fn select_all(&self, window: &mut Window, cx: &mut App) {
        match &self.kind {
            Kind::Line(state) => state.update(cx, |state, cx| state.select_all(window, cx)),
            Kind::Lines(state, _) => state.update(cx, |state, cx| state.select_all(window, cx)),
        }
    }
}

#[cfg(test)]
mod tests;
