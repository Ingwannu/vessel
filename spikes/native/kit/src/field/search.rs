//! Paseo's standalone `SearchField` on the existing one-line editor engine.
//!
//! [Decision Log]
//! - Purpose: catalogue and model filters share one search surface and clear.
//! - Constraint: source `search-field.tsx` owns its Search/X marks and clearing;
//!   ordinary form chrome has different geometry and colors.
//! - Choice: a typed wrapper constructs only `TextField`'s one-line engine and
//!   reuses its bare renderer, focus, edit/IME and change-event path.
//! - Alternative: an area wrapper, a second input engine, or a method accepting
//!   multiline `TextField` values and silently rendering an incompatible search shape.
//! - Why/cost: no alternate input implementation or caller-owned clear glyph;
//!   existing `TextField`/form/composer APIs stay unchanged. Native accessibility
//!   naming remains the existing kit-wide gap, not a claimed OS parity result.
use super::{Kind, TextField};
use crate::{
    Tokens,
    assets::Glyph,
    glyph,
    theme::{radius, spacing},
};
use gpui::prelude::*;
use gpui::{App, Context, Div, ElementId, MouseButton, SharedString, Stateful, Window, div, px};
const MAX_WIDTH: f32 = 420.0;
const ICON: f32 = 14.0;
const INPUT_HEIGHT: f32 = 20.0;
const SLOP: f32 = spacing::S2;

/// One-line filter with its own source-defined search mark and clear target.
#[derive(Debug, Clone)]
pub struct SearchField {
    editor: TextField,
}
impl SearchField {
    /// Enter does not submit a filter; all edits use the existing change path.
    pub fn new<V: 'static>(
        placeholder: &str,
        on_change: fn(&mut V, String, &mut Context<'_, V>),
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        Self {
            editor: TextField::with_change(placeholder, ignore_submit::<V>, on_change, window, cx),
        }
    }
    /// Current text for the owning view's shared matcher.
    #[must_use]
    pub fn text(&self, cx: &App) -> String {
        self.editor.text(cx)
    }
    /// Locale changes retain the person's query and selection.
    pub fn set_placeholder(&self, placeholder: &str, window: &mut Window, cx: &mut App) {
        self.editor.set_placeholder(placeholder, window, cx);
    }
    /// Navigation may explicitly reset this same real editor.
    pub fn clear(&self, window: &mut Window, cx: &mut App) {
        self.editor.clear(window, cx);
    }
    /// Draw the complete source rail field; callers add no icon/clear chrome.
    #[must_use]
    pub fn element(
        &self,
        id: impl Into<ElementId>,
        tokens: &Tokens,
        cx: &mut App,
    ) -> Stateful<Div> {
        let id = id.into();
        let focus = self.editor.clone();
        let clear = self.editor.clone();
        let has_text = !self.editor.text(cx).is_empty();
        frame(self.editor.focused.get(), tokens)
            .id(id.clone())
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                focus.focus(window, cx);
            })
            .child(glyph(Glyph::Search, ICON, tokens.neutral.foreground_muted).flex_none())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(INPUT_HEIGHT))
                    .child(self.editor.bare(tokens, cx)),
            )
            .when(has_text, |field| {
                field.child(
                    clear_target(
                        ElementId::NamedChild(std::sync::Arc::new(id), SharedString::from("clear")),
                        tokens,
                    )
                    .on_click(move |_, window, cx| {
                        clear_from_control(&clear, window, cx);
                        clear.focus(window, cx);
                        cx.stop_propagation();
                    }),
                )
            })
    }
}
// Programmatic TextField::clear deliberately suppresses Change. A person
// using X must instead take the editor's ordinary replacement path, so the
// owning filter sees the empty value exactly as it sees keyboard edits.
fn clear_from_control(editor: &TextField, window: &mut Window, cx: &mut App) {
    match &editor.kind {
        Kind::Line(state) => state.update(cx, |state, cx| state.replace_all("", window, cx)),
        Kind::Lines(state, _) => state.update(cx, |state, cx| state.replace_all("", window, cx)),
    }
}
fn ignore_submit<V: 'static>(_: &mut V, _: String, _: &mut Window, _: &mut Context<'_, V>) {}
fn frame(focused: bool, tokens: &Tokens) -> Div {
    let n = tokens.neutral;
    div()
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .flex_1()
        .min_w_0()
        .max_w(px(MAX_WIDTH))
        .py(px(spacing::S1_5))
        .px(px(spacing::S3))
        .rounded(px(radius::MD))
        .border_1()
        .border_color(if focused { n.border_accent } else { n.border })
        .bg(if focused { n.surface2 } else { n.surface1 })
}
fn clear_target(id: ElementId, tokens: &Tokens) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(|| "search-field-clear".to_owned())
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .m(px(SLOP.copysign(-1.0)))
        .p(px(SLOP))
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(glyph(Glyph::Remove, ICON, tokens.neutral.foreground_muted).flex_none())
}
#[cfg(test)]
mod tests;
