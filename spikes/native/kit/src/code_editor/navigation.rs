//! Scalar selection/reveal uses the existing rope engine and clamps both
//! line and column to the loaded document. A stale navigation target must
//! never move beyond its final line or split a UTF-8 scalar.

use super::CodeEditor;
use gpui::{App, Window};
use gpui_base::input::{EditorState, RopeExt as _};
use std::ops::Range;

/// Engine coordinates: zero-based line and Unicode scalar column.
/// File owners adapt their validated one-based protocol positions here.
pub use gpui_base::input::Position as SourcePoint;

impl CodeEditor {
    /// Select/reveal a source range; equal endpoints reveal a caret.
    pub fn reveal_range(&self, range: &Range<SourcePoint>, window: &mut Window, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            let start = offset(state, &range.start);
            let end = offset(state, &range.end).max(start);
            state.set_selected_range(start..end, cx);
            state.focus(window, cx);
        });
    }
}

fn offset(state: &EditorState, point: &SourcePoint) -> usize {
    let text = state.text();
    let last = u32::try_from(text.lines_len().saturating_sub(1)).unwrap_or(u32::MAX);
    text.position_to_offset(&SourcePoint {
        line: point.line.min(last),
        character: point.character,
    })
}
