//! Muted hunk headers using the same engine's existing text decorations.
//!
//! [Decision Log]
//! - Purpose: Paseo diff-document/palette.ts gives headers foregroundMuted.
//! - Existing assets: the editor's UTF-8 text-decoration collection overlays
//!   syntax without changing text, selection, metrics or input handling.
//! - Choice: retain one immutable header-range collection per readonly buffer;
//!   replace only its foreground when that semantic theme colour changes.
//! - Why: a new syntax kind would mix diff structure into the source lexer;
//!   a vendor text-colour hook would duplicate an existing public API.
//! - Cost: one range scan at admission and one update per actual colour change.
//!   Clones share the collection and colour cache, avoiding redraw-notify loops.
use super::{DiffText, DiffTone};
use gpui::{App, Entity, HighlightStyle, Hsla};
use gpui_base::input::{EditorState, TextDecoration, TextDecorationCollection};
use std::{cell::Cell, ops::Range, rc::Rc, sync::Arc};

#[derive(Debug, Clone)]
pub(super) enum HeaderInk {
    None,
    Rows {
        ranges: Arc<[Range<usize>]>,
        collection: TextDecorationCollection,
        colour: Rc<Cell<Option<Hsla>>>,
    },
}
impl HeaderInk {
    pub(super) fn new(state: &Entity<EditorState>, text: &DiffText, cx: &mut App) -> Self {
        let ranges = ranges(text);
        if ranges.is_empty() {
            return Self::None;
        }
        let collection = state.update(cx, |state, cx| {
            state.create_decorations_collection(Vec::new(), cx)
        });
        Self::Rows {
            ranges: ranges.into(),
            collection,
            colour: Rc::new(Cell::new(None)),
        }
    }

    pub(super) fn apply(&self, ink: Hsla, cx: &mut App) {
        let Self::Rows {
            ranges,
            collection,
            colour,
        } = self
        else {
            return;
        };
        if colour.get() == Some(ink) {
            return;
        }
        // Cache before notify: an observing owner may redraw this same clone.
        colour.set(Some(ink));
        let style = HighlightStyle {
            color: Some(ink),
            ..HighlightStyle::default()
        };
        collection.set(
            ranges
                .iter()
                .map(|range| TextDecoration::new(range.clone(), style))
                .collect(),
            cx,
        );
    }
}

fn ranges(text: &DiffText) -> Vec<Range<usize>> {
    let mut offset = 0_usize;
    text.text
        .split('\n')
        .zip(text.rows.iter())
        .filter_map(|(line, row)| {
            let start = offset;
            let end = start.saturating_add(line.len());
            // Input admission already pairs all rows; saturation keeps the
            // terminal offset safe even at the address-space bound.
            offset = end.saturating_add(1);
            (row.tone == DiffTone::Header && !line.is_empty()).then_some(start..end)
        })
        .collect()
}

#[cfg(test)]
mod tests;
