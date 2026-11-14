//! A branch's name on one line, everywhere this area draws one (the
//! Changes header, the commit form, the switcher's rows): cut in its
//! middle to the room it has, the whole name in a tooltip where the place
//! can carry one.
//!
//! [Decision Log]
//! - Purpose: two long branches stay told apart (their distinguishing
//!   parts are often at both ends: `origin/` at the start, the topic at
//!   the end), and a long name never makes a toolbar several lines tall.
//! - Constraints: GPUI cuts text only at its end; the room is known only
//!   from layout. Paseo's `ToolbarLabelSelectTrigger` and `ComboboxItem`
//!   cut at the end (`numberOfLines={1}`).
//! - Alternatives: wrapping (the first slice: three lines, broken
//!   mid-word); the end cut of Paseo (hides `…-레이아웃` vs `…-v2`).
//! - Choice: the kit's middle cut (`fonts::elide_middle`) to the width a
//!   probe measured on the name's own flexible box in the previous frame;
//!   before the first measure, the caller's estimate.
//! - Why: the coordinator's rule for a path applies to a branch, the
//!   reference's one line kept.
//! - Cost: the first frame uses the estimate, and asks for one more
//!   frame so the measured room replaces it at once (the phone's capture
//!   showed the estimate's cut, end-truncated again by the box, until the
//!   five-second refresh redrew). One frame, once per probe: nothing
//!   repeats while the screen is still.

use vessel_app_kit::Type;
use vessel_app_kit::fonts::elide_middle;
use vessel_app_kit::tooltip::Place;
use vessel_app_kit::ui::Window;

/// A name as it fits, and whether it was cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OneLine {
    /// What is drawn.
    pub(super) text: String,
    /// Part of it is hidden: the whole goes in the tooltip.
    pub(super) cut: bool,
}

/// `name` in `face`, cut in its middle to `room` points.
pub(super) fn one_line(window: &Window, name: &str, face: Type, room: f32) -> OneLine {
    let text = elide_middle(window, name, face, room.max(0.0));
    let cut = text != name;
    OneLine { text, cut }
}

/// The room for a name in the box `place` measured, less `chrome` (the
/// box's own insets and glyphs); `estimate` until it has been measured,
/// with the next frame asked for so the measure is used at once.
pub(super) fn room(place: &Place, chrome: f32, estimate: f32, window: &Window) -> f32 {
    place.get().map_or_else(
        || {
            window.request_animation_frame();
            estimate
        },
        |bounds| chrome.mul_add(-1.0, f32::from(bounds.size.width)),
    )
}
