//! Numeric settings-field chrome, following Paseo's appearance-section.tsx.
//!
//! [Decision Log]
//! - Purpose: font sizes and content widths are edited as numbers, not selects.
//! - Constraints: reuse the existing input engine; ordinary form fields retain
//!   their geometry. Parsing and clamping belong to view, not this control.
//! - Alternatives: styling each settings row, or changing every `TextField`.
//! - Choice: one shell with typed 64/80 widths, blur/submit and select-on-focus.
//! - Costs: the caller owns its draft and resynchronizes external changes.

use super::{Kind, TextField, project};
use crate::theme::spacing;
use crate::{Tokens, Type, TypeExt as _};
use gpui::prelude::*;
use gpui::{AnyElement, App, Context, Window, div, px};
use gpui_base::input::{Input, InputEvent};

/// Paseo's closed numeric-field sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberWidth {
    /// Font size, 64 points.
    FontSize,
    /// Content width, 80 points.
    ContentWidth,
}
impl NumberWidth {
    /// Reference width in logical points.
    #[must_use]
    pub const fn points(self) -> f32 {
        match self {
            Self::FontSize => 64.0,
            Self::ContentWidth => 80.0,
        }
    }
}

/// A numeric draft edited by the same engine as ordinary text fields.
#[derive(Debug, Clone)]
pub struct NumberField {
    field: TextField,
    width: NumberWidth,
}
impl NumberField {
    /// Commit on Enter or blur; report edits without deciding their meaning.
    pub fn new<V: 'static>(
        width: NumberWidth,
        on_commit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
        on_edit: fn(&mut V, String, &mut Window, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let field = TextField::new("", on_commit, window, cx);
        if let Kind::Line(state) = &field.kind {
            // The engine owns caret alignment; container text alignment alone
            // would leave typed digits and their selection at the left edge.
            state.update(cx, |state, cx| {
                state.set_text_align(gpui::TextAlign::Right, cx);
            });
            cx.subscribe_in(
                state,
                window,
                move |owner, state, event: &InputEvent, window, cx| match event {
                    InputEvent::Focus => {
                        let input = state.clone();
                        // Finish the pointer's initial caret placement before
                        // applying selectTextOnFocus, including touch focus.
                        window.defer(cx, move |window, cx| {
                            input.update(cx, |state, cx| state.select_all(window, cx));
                        });
                    }
                    InputEvent::Blur => {
                        on_commit(owner, state.read(cx).value().to_string(), window, cx);
                    }
                    InputEvent::Change => {
                        on_edit(owner, state.read(cx).value().to_string(), window, cx);
                    }
                    InputEvent::PressEnter { .. } => {}
                },
            )
            .detach();
        }
        Self { field, width }
    }

    /// Replace the draft after a commit or an external preference update.
    pub fn set_text(&self, text: &str, window: &mut Window, cx: &mut App) {
        if self.field.text(cx) != text {
            self.field.set_text(text, window, cx);
        }
    }

    /// Current draft, before parsing in view.
    #[must_use]
    pub fn text(&self, cx: &App) -> String {
        self.field.text(cx)
    }

    /// Paseo's 36-point minimum on both layouts; the caller draws the unit.
    #[must_use]
    pub fn element(&self, tokens: &Tokens, cx: &mut App) -> AnyElement {
        let Kind::Line(state) = &self.field.kind else {
            return div().into_any_element();
        };
        let neutral = tokens.neutral;
        project(state, tokens, neutral.foreground_muted, cx);
        super::settings_frame::frame(state)
            .flex()
            .items_center()
            .w(px(self.width.points()))
            .min_h(px(36.0))
            .px(px(spacing::S3))
            .py(px(spacing::S2))
            .rounded(px(6.0))
            .border_1()
            .border_color(neutral.border)
            .bg(neutral.surface2)
            .text_color(neutral.foreground)
            .type_(Type::Body)
            .text_align(gpui::TextAlign::Right)
            .child(div().flex_1().child(Input::new(state)))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
