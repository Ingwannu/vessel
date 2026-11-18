//! The list inside a flyout, a page beside its menu (TRACKER 791): the
//! list's panel, placed by the flyout. Apart from `picker.rs` for its size.

use gpui::prelude::*;
use gpui::{AnyElement, Context, ElementId, Window, div};

use super::{ComboHandlers, ComboHeader, ComboRow, ComboboxState, panel};
use crate::theme::Tokens;

/// The list inside a flyout, a page beside its menu ([`crate::submenu`]):
/// the same panel, placed by [`crate::submenu::flyout`]; a press outside it
/// is its menu's to judge.
#[expect(
    clippy::too_many_arguments,
    reason = "a list is drawn from its state, its content, its handlers and the window's size"
)]
pub fn combobox_inline<V: 'static>(
    id: impl Into<ElementId>,
    state: &ComboboxState,
    header: &ComboHeader,
    rows: &[ComboRow],
    handlers: &ComboHandlers<V>,
    width: f32,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, V>,
) -> Option<AnyElement> {
    if !state.open {
        return None;
    }
    let list = panel::List {
        id: id.into(),
        state,
        header,
        rows,
        handlers,
        width,
    };
    let drawn = panel::panel(&list, panel::Surface::Flyout, tokens, window, cx);
    let tips = state.tips.render(tokens);
    Some(div().child(drawn).children(tips).into_any_element())
}
