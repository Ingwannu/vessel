//! The conversation's motion, quick and fine (the owner on 0.0.0.0.13:
//! "부드럽지만 빠르게 … 설정에서 끌 수도 있고 그럼 바로 딱딱 가게"): a
//! message sent arrives, a reply grows in. One place for the durations and
//! the curve until the lead's motion tokens and setting land: the Codex
//! app's 150 ms and 300 ms on `cubic-bezier(.23, 1, .32, 1)` (GPUI's
//! `ease_out_quint`, the same curve).
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
//!   drawn at its end state. Off
//!   (`VESSEL_APP_MOTION=off`, until the setting exists): the end state at
//!   once. Nothing here repeats: an idle session draws no frame for it.
//! - Cost: a reduced-motion wish of the system and a software renderer
//!   are not detected yet (the lead's setting will carry them).

use std::time::Duration;

use gpui::{Animation, AnimationExt as _};
use vessel_app_kit::ui::{
    AnyElement, ElementId, IntoElement as _, ParentElement as _, Styled as _, div, px,
};

/// A message sent: Codex's quick step.
pub(super) const FAST: Duration = Duration::from_millis(150);

/// A reply growing in: Codex's longer step.
pub(super) const SLOW: Duration = Duration::from_millis(300);

/// How long after a batch its rows still count as arriving.
pub(super) const FRESH: Duration = Duration::from_millis(400);

/// How far an arriving row rises, in points, and its negative (the
/// curve's slope: no float arithmetic beside `mul_add`).
const RISE: f32 = 6.0;
const FALL: f32 = -6.0;

/// Whether the conversation moves.
pub(super) fn on() -> bool {
    std::env::var("VESSEL_APP_MOTION").as_deref() != Ok("off")
}

/// The person's own message settling: drawn whole on its first frame
/// (Enter to the message drawn is the next frame, slice M's measure), it
/// only rises `RISE` to its place over `duration`; at once when motion
/// is off. A fade would leave the first frames nearly empty.
pub(super) fn settle(id: ElementId, duration: Duration, element: AnyElement) -> AnyElement {
    if !on() {
        return element;
    }
    div()
        .child(element)
        .with_animation(
            id,
            Animation::new(duration).with_easing(gpui::ease_out_quint()),
            |row, delta| row.mt(px(delta.mul_add(FALL, RISE))),
        )
        .into_any_element()
}

/// `element` arriving: from transparent and `RISE` lower to its place,
/// over `duration` on Codex's curve; at once when motion is off.
pub(super) fn arrive(id: ElementId, duration: Duration, element: AnyElement) -> AnyElement {
    if !on() {
        return element;
    }
    div()
        .child(element)
        .with_animation(
            id,
            Animation::new(duration).with_easing(gpui::ease_out_quint()),
            |row, delta| row.opacity(delta).mt(px(delta.mul_add(FALL, RISE))),
        )
        .into_any_element()
}
