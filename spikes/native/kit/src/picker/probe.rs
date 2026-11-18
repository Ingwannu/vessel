//! Where an element is drawn (moved out of `picker.rs`, which may not grow,
//! when the rows gained their separator, TRACKER 807).

use std::{cell::Cell, rc::Rc};

use gpui::prelude::*;
use gpui::{AnyElement, Bounds, Pixels, canvas};

/// Where an element is drawn, recorded in `cell` every frame (an absolute
/// layer over its parent): the anchor of a list opened from it later
/// ([`super::ComboboxState::set_anchor`]), when several triggers share one
/// list.
#[must_use]
pub fn anchor_probe(cell: &Rc<Cell<Option<Bounds<Pixels>>>>) -> AnyElement {
    let cell = Rc::clone(cell);
    canvas(
        move |bounds, _window, _cx| cell.set(Some(bounds)),
        |_, (), _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
    .into_any_element()
}
