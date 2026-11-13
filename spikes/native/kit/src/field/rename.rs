//! Rename-modal input chrome and editing capabilities on the existing engine.
//!
//! [Decision Log]
//! - Purpose: Paseo rename-modal.tsx has surface0/pad12/r6, distinct from a form
//!   and from Appearance's numeric inputs; saving rejects all user edits.
//! - Constraints: the original `TextField` entries and geometry stay intact, and
//!   every field still uses `line()`/`register` so IME-aware shortcuts see it.
//! - Alternatives: restyling in each dialog, or writing a second editing engine.
//! - Choice: a profile plus typed editability and selection on `TextField`.
//! - Costs: callers own the pending state and restore editing after a failure.
use super::{Kind, TextField, project};
use crate::theme::spacing;
use crate::{Tokens, Type, TypeExt as _};
use gpui::prelude::*;
use gpui::{AnyElement, App, Window, div, px};
use gpui_base::input::Input;

/// Whether a field accepts user changes. Read-only still permits selection/copy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Editability {
    /// Normal typing, paste and input-method composition.
    #[default]
    Editable,
    /// A pending operation keeps its submitted text until it settles.
    ReadOnly,
}
impl TextField {
    /// Reject user edits while pending, including paste and IME replacements.
    /// Programmatic `set_text` remains available to restore a draft on failure.
    pub fn set_editability(&self, mode: Editability, cx: &mut App) {
        let readonly = mode == Editability::ReadOnly;
        match &self.kind {
            Kind::Line(state) => state.update(cx, |state, cx| state.set_readonly(readonly, cx)),
            Kind::Lines(state, _) => state.update(cx, |state, cx| state.set_readonly(readonly, cx)),
        }
    }
    /// Select the current value when a rename modal opens. Call after focus.
    pub fn select_all(&self, window: &mut Window, cx: &mut App) {
        match &self.kind {
            Kind::Line(state) => state.update(cx, |state, cx| state.select_all(window, cx)),
            Kind::Lines(state, _) => state.update(cx, |state, cx| state.select_all(window, cx)),
        }
    }
    /// Paseo's one-line rename profile. Construct with the ordinary `new()` or
    /// `with_change()` path, so the shared focused-composing registry sees it.
    #[must_use]
    pub fn rename_element(&self, tokens: &Tokens, cx: &mut App) -> AnyElement {
        let Kind::Line(state) = &self.kind else {
            return div().into_any_element();
        };
        let neutral = tokens.neutral;
        project(state, tokens, neutral.foreground_muted, cx);
        super::settings_frame::frame(state)
            .w_full()
            .flex()
            .items_center()
            .p(px(spacing::S3))
            .rounded(px(6.0))
            .border_1()
            .border_color(neutral.border)
            .bg(neutral.surface0)
            .type_(Type::Body)
            .text_color(neutral.foreground)
            .line_height(px(crate::fonts::ui(20.0)))
            .child(div().flex_1().min_w_0().child(Input::new(state)))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
