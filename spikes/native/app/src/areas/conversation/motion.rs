//! The conversation's motion, quick and fine (the owner on 0.0.0.0.13:
//! "부드럽지만 빠르게 … 설정에서 끌 수도 있고 그럼 바로 딱딱 가게"): a
//! message sent settles into place, a reply grows in. Lengths and the
//! curve are the kit's tokens (`vessel_app_kit::motion`, the lead's: a
//! message arriving is a surface, `STANDARD`, 300 ms on
//! `cubic-bezier(.23, 1, .32, 1)`).
//!
//! [Decision Log]
//! - Purpose: rows popped in at once; the owner asked for motion that
//!   helps the eye follow, and for a way to turn it off.
//! - Constraint: a list row is built again every frame and dropped when
//!   scrolled out of view, so an animation keyed by a row would play again
//!   each time the row came back.
//! - Choice: only what just arrived moves: the pending message as it is
//!   sent (whole from its first frame, only rising: the person's Enter
//!   is answered on the next frame), a reply when its entry first comes
//!   ([`FRESH`] after the batch that brought it); everything else is
//!   drawn at its end state. Both are one-shot GPUI transitions, so while
//!   motion is held (Animations off, the system's reduced motion, a
//!   software renderer: `app::motion`) GPUI draws their end at once.
//!   Nothing here repeats: an idle session draws no frame for it.

use std::time::Duration;

use gpui::AnimationExt as _;
use vessel_app_kit::motion::{STANDARD, transition};
use vessel_app_kit::ui::{
    AnyElement, ElementId, IntoElement as _, ParentElement as _, Styled as _, div, px,
};

/// How long after a batch its rows still count as arriving: the
/// transition and a little slack for the frame that draws it.
pub(super) const FRESH: Duration = STANDARD.saturating_add(Duration::from_millis(100));

/// How far an arriving row rises, in points, and its negative (the
/// curve's slope: no float arithmetic beside `mul_add`).
const RISE: f32 = 6.0;
const FALL: f32 = -6.0;

/// The person's own message settling: drawn whole on its first frame
/// (Enter to the message drawn is the next frame, slice M's measure), it
/// only rises `RISE` to its place. A fade would leave the first frames
/// nearly empty.
pub(super) fn settle(id: ElementId, element: AnyElement) -> AnyElement {
    div()
        .child(element)
        .with_animation(id, transition(STANDARD), |row, delta| {
            row.mt(px(delta.mul_add(FALL, RISE)))
        })
        .into_any_element()
}

/// A reply arriving: from transparent and `RISE` lower to its place.
pub(super) fn arrive(id: ElementId, element: AnyElement) -> AnyElement {
    div()
        .child(element)
        .with_animation(id, transition(STANDARD), |row, delta| {
            row.opacity(delta).mt(px(delta.mul_add(FALL, RISE)))
        })
        .into_any_element()
}
