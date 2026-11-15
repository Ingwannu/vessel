//! Full-height readonly files in one owner-scrolled document, same input engine.
//!
//! [Decision Log]
//! - Existing assets: `DiffEditor`, the engine's complete display map and owner
//!   `VirtualList::reveal_bounds`. No independent cursor/selection/wrap model.
//! - Choice: keep full file height, remove source-panel vertical insets and the
//!   editor's empty bottom rows. Patch7 bounds shaping against the parent;
//!   its installed compilation and runtime proof are pending.
//! - Why: nested fixed-height cards break reading a multi-file diff as one
//!   document. The parent also owns keyboard reveal, including offclip rows.
//! - Cost: a retained observation reports changed focused caret positions;
//!   callers keep its exact ticket guard and invalidate their scrolling owner.
use super::{DiffEditor, DiffText, DiffWrapping};
use crate::code_editor::LineSyntax;
use crate::{Tokens, Type};
use gpui::prelude::*;
use gpui::{AnyElement, App, Bounds, Context, Pixels, Subscription, Window, div, point, px, size};
use gpui_base::input::{BufferPoint, EditorState, RopeExt as _};
use std::rc::Rc;

/// One full-height readonly file. Clones share the engine and reveal observer.
#[derive(Clone)]
pub struct DiffDocumentEditor {
    editor: DiffEditor,
    _reveal: Rc<Subscription>,
}
impl std::fmt::Debug for DiffDocumentEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiffDocumentEditor").finish_non_exhaustive()
    }
}
impl DiffDocumentEditor {
    /// The owner reveals these window-coordinate caret bounds in its one list.
    /// Callbacks must retain and recheck the original pane/file generation.
    pub fn new<V: 'static, S: LineSyntax>(
        text: DiffText,
        syntax: S,
        wrapping: DiffWrapping,
        mut on_reveal: impl FnMut(&mut V, Bounds<Pixels>, &mut Window, &mut Context<'_, V>) + 'static,
        window: &mut Window,
        cx: &mut Context<'_, V>,
    ) -> Self {
        let editor = DiffEditor::new(text, syntax, wrapping, window, cx);
        editor.editor.state.update(cx, |state, cx| {
            state.set_editor_paddings(gpui::Edges {
                left: px(super::super::GUTTER_LEAD),
                ..gpui::Edges::default()
            });
            state.set_scroll_beyond_last_line(Some(0), window, cx);
        });
        let mut previous = None;
        let observer = cx.observe_in(
            &editor.editor.state,
            window,
            move |owner, state, window, cx| {
                let state = state.read(cx);
                if !state.has_selection_focus(window, cx) || previous == Some(state.cursor()) {
                    return;
                }
                let Some(bounds) = caret_bounds(state) else {
                    return;
                };
                previous = Some(state.cursor());
                on_reveal(owner, bounds, window, cx);
            },
        );
        Self {
            editor,
            _reveal: Rc::new(observer),
        }
    }

    /// Reflow the existing buffer; split consumers explicitly choose Scroll.
    pub fn set_wrapping(&self, wrapping: DiffWrapping, window: &mut Window, cx: &mut App) {
        self.editor.set_wrapping(wrapping, window, cx);
    }

    /// Full content height. Parent clipping never changes the document's extent.
    pub fn element(
        &self,
        tokens: &Tokens,
        width: Pixels,
        window: &Window,
        cx: &mut App,
    ) -> AnyElement {
        let (font_size, line_height, weight) = Type::Mono.metrics();
        let gutter = crate::fonts::measure(window, &self.editor.sample, Type::Mono);
        let font = gpui::Font {
            family: Type::Mono.family(),
            features: gpui::FontFeatures::default(),
            fallbacks: Some(crate::fonts::fallbacks()),
            weight,
            style: gpui::FontStyle::Normal,
        };
        self.editor.editor.state.update(cx, |state, cx| {
            // Six is the existing mapped gutter's right margin, not a new gap.
            state.prewrap(font, px(font_size), text_width(width, gutter), cx);
        });
        let rows = self
            .editor
            .editor
            .state
            .read(cx)
            .display_map()
            .wrap_row_count()
            .max(self.editor.rows.len());
        div()
            .w_full()
            .h(row_height(rows, px(line_height)))
            .flex_none()
            .child(self.editor.element(tokens, cx))
            .into_any_element()
    }
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite pixel geometry from the admitted file and current font metrics"
)]
fn row_height(rows: usize, line_height: Pixels) -> Pixels {
    line_height * rows
}
#[expect(
    clippy::arithmetic_side_effects,
    reason = "viewport pixel geometry is clamped before engine wrapping"
)]
fn text_width(width: Pixels, gutter: f32) -> Pixels {
    (width - px(super::super::GUTTER_LEAD) - px(gutter) - px(6.)).max(px(1.))
}
#[expect(
    clippy::arithmetic_side_effects,
    reason = "window-coordinate caret geometry uses the existing complete display map"
)]
fn caret_bounds(state: &EditorState) -> Option<Bounds<Pixels>> {
    let input = state.input_bounds();
    if input.size.height <= px(0.) {
        return None;
    }
    let buffer = state.text().offset_to_point(state.cursor());
    let display = state
        .display_map()
        .buffer_pos_to_display_pos(BufferPoint::new(buffer.row, buffer.column));
    let (_, line_height, _) = Type::Mono.metrics();
    Some(Bounds::new(
        input.origin + point(px(0.), row_height(display.row, px(line_height))),
        size(input.size.width, px(line_height)),
    ))
}

#[cfg(test)]
#[path = "document_tests.rs"]
mod tests;
