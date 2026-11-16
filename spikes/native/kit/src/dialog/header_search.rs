//! Paseo's sheet-header search row, drawn by the one dialog header from the
//! identified search engine's unboxed input (shared-dialog-header-search.md).

use crate::assets::Glyph;
use crate::theme::{icon, spacing};
use gpui::prelude::*;
use gpui::{AnyElement, Div, Hsla, div, px};

/// Paseo's sheet-header search (`SheetHeader.search`, adaptive-modal-sheet.tsx
/// 443-461): only `SearchField::header_search` makes one, so the row always
/// wraps that identified one-line engine, never a second input.
pub struct HeaderSearch {
    field: AnyElement,
}

impl HeaderSearch {
    pub(crate) fn new(field: AnyElement) -> Self {
        Self { field }
    }
}

impl std::fmt::Debug for HeaderSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeaderSearch").finish_non_exhaustive()
    }
}

/// Paseo's `searchRow` (adaptive-modal-sheet.tsx 149-155, 443-461): unboxed,
/// a 16-point muted Search mark, gap 2, the dialog's 24 inset and 12 below,
/// with no clear control, fill or width cap, on compact and desktop alike.
pub(super) fn search_row(search: HeaderSearch, inset: f32, mark: Hsla) -> Div {
    div()
        .debug_selector(|| "dialog-header-search".to_owned())
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .px(px(inset))
        .pb(px(spacing::S3))
        .child(crate::glyph(Glyph::Search, icon::MD, mark).flex_none())
        .child(search.field)
}
