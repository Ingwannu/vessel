//! How far a press moves before a drag starts.
//!
//! [Decision Log]
//! - Purpose: a press on a row that can be dragged is still a press until
//!   the pointer has clearly moved. Paseo 0.15.4's web draggable list
//!   (`components/draggable-list.web.tsx`, its pointer sensor's
//!   `distance: 6`) waits 6 points; GPUI starts any drag after 2, so a
//!   slightly shaky click on a sidebar row picked the row up.
//! - Constraint: GPUI's distance is one constant inside `div.rs`, read by
//!   every element; nothing outside GPUI can change it.
//! - Alternatives: the row's own press-and-move tracking in place of
//!   `on_drag` (a second drag model beside GPUI's, with its own drop
//!   targets and drag view); one distance for the whole app (changes every
//!   existing drag, such as the queue's rows, at once).
//! - Choice: GPUI's per-element `drag_threshold`, added in the vendored
//!   copy (`spikes/release/patches/README.md`, "gpui-pre"); an element
//!   that names none keeps GPUI's 2. Lists that follow Paseo's draggable
//!   list name [`LIST_DRAG_DISTANCE`].
//! - Cost: the GPUI core is a vendored crate to carry at each pin.

use gpui::{Pixels, px};

/// Paseo's draggable list: how far a press moves before its row is
/// dragged (`StatefulInteractiveElement::drag_threshold`).
pub const LIST_DRAG_DISTANCE: Pixels = px(6.0);

#[cfg(test)]
mod tests;
