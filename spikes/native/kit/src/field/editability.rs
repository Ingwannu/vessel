//! Hold a submitted form's real editor without losing focus or selection.
//! A callback guard alone leaves the visible editor ahead of the submitted draft.

use super::{Kind, TextField};
use gpui::App;

/// Whether the person may change the text while an owning form waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Editability {
    /// Normal text entry, including input methods and paste.
    Editable,
    /// Retain focus, selection and copy; reject user edits until retry.
    ReadOnly,
}

impl TextField {
    /// Hold the editor itself, so an in-flight answer and its visible text
    /// cannot diverge. Programmatic restoration remains available.
    pub fn set_editability(&self, editability: Editability, cx: &mut App) {
        let readonly = editability == Editability::ReadOnly;
        match &self.kind {
            Kind::Line(state) => state.update(cx, |state, cx| state.set_readonly(readonly, cx)),
            Kind::Lines(state, _) => state.update(cx, |state, cx| state.set_readonly(readonly, cx)),
        }
    }
}

#[cfg(test)]
mod tests;
