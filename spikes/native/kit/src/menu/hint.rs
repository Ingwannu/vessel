//! An ordinary row's hint at its end, muted (Paseo's `DropdownMenuItem`
//! `hint`, `menuItemHint`: `foregroundMuted` at `fontSize.sm`, 12).
//!
//! [Decision Log]
//! - Purpose: TRACKER 789. Paseo's tab menu shows the agent id's first
//!   seven characters after Copy agent id; the kit's ordinary row drew only
//!   a shortcut there.
//! - Choice: the item's `value`, which a submenu row already draws muted
//!   before its chevron (`sub_row`), is drawn at an ordinary row's end too,
//!   pushed there as the shortcut is (`marginLeft: auto`). No second field
//!   for one muted word.
//! - Cost: a row with both a shortcut and a value shows both, the value
//!   first; no caller has both.

use super::MenuItem;
use crate::fonts::Type;
use crate::popup::Surfaces;
use crate::widgets::TypeExt as _;
use gpui::prelude::*;
use gpui::{Div, div, px};

/// The row's hint, when it has one and opens no page.
pub(super) fn hint(index: usize, item: &MenuItem, surfaces: &Surfaces) -> Option<Div> {
    if item.opens.is_some() {
        return None;
    }
    let value = item.value.clone()?;
    Some(
        div()
            .debug_selector(move || format!("menu-row-hint-{index}"))
            .ml_auto()
            .flex_none()
            .pl(px(crate::theme::spacing::S2))
            .type_(Type::Meta)
            .text_color(surfaces.foreground_muted)
            .child(value),
    )
}
