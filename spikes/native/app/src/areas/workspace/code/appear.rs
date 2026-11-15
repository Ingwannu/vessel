//! What the code panel shows because of something the person did, coming
//! in: the editor's dirty dot, "Saving...", "Save failed", the conflict
//! bar, the selection's "Add lines … to chat" (the owner's word 39: a
//! short transition on every change a person causes, the end state at
//! once while motion is held, nothing moving continuously).
//!
//! [Decision Log]
//! - Purpose: these marks popped in on the frame their state changed.
//! - Constraint: the panel is built again every frame; GPUI keeps an
//!   animation's state by the element's id for as long as the element is
//!   drawn on consecutive frames, and drops it on the first frame without
//!   it.
//! - Choice: a one-shot GPUI transition keyed by an id that names the mark
//!   (not its text), so the mark plays once when it first appears and is
//!   drawn at its end on every frame after; it plays again only when it
//!   comes back after a frame without it, which is when it is arriving
//!   again. A mark's own change (a dot, a word) is `QUICK`; a bar that
//!   arrives under the toolbar is a surface, `STANDARD` (the kit's motion
//!   tokens, `vessel_app_kit::motion`).
//! - Alternatives: keeping a "shown since" instant per mark in the model
//!   (more state, and the same result); a height transition for the bar
//!   (the editor under it would move with it on every frame of the
//!   transition, a larger repaint for no gain in reading).
//! - Cost: leaving is not animated: a mark that goes is gone at once.
//!   While motion is held GPUI draws the end at once (`app::motion`).

use std::time::Duration;

use gpui::AnimationExt as _;
use vessel_app_kit::motion::transition;
use vessel_app_kit::ui::{AnyElement, ElementId, IntoElement, Styled};

/// `element` fading in over `length`, once, keyed by `id`.
pub(super) fn appear<E>(id: impl Into<ElementId>, length: Duration, element: E) -> AnyElement
where
    E: IntoElement + Styled + 'static,
{
    element
        .with_animation(id, transition(length), |element, delta| {
            element.opacity(delta)
        })
        .into_any_element()
}
