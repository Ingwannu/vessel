//! A list over the composer that opens and closes with a short transition
//! (the owner's word 39: every change a person causes moves, smooth and
//! quick; with animations off it is there at once).
//!
//! [Decision Log]
//! - Purpose: the `/` menu (and the `@` menu after it) appeared and went
//!   in one frame, as Paseo's (`components/ui/autocomplete-popover.tsx`
//!   mounts and unmounts its popover with no animation).
//! - Constraint: one set of motion tokens for the whole app (the lead's;
//!   for now `slider::motion`: the `SLIDE` curve, `DRAG_GLIDE` 0.15 s);
//!   nothing may animate while nothing changes (idle CPU is reported per
//!   slice); reduced motion, the app's own setting and a software renderer
//!   all mean the end state at once (GPUI's `reduce_motion`).
//! - Alternatives: GPUI's `with_animation` on the list (plays on the way
//!   in only: a closed menu is no longer drawn, so it cannot fade out);
//!   the kit's list owning the timing (it does not know when its screen
//!   stops drawing it, and the kit cannot read the view's tokens).
//! - Choice: the screen keeps how open the list is as a [`Tween`] from 0
//!   (gone) to 1 (open), glides it when the menu opens or closes, draws
//!   the list while it is above 0 (its last rows while it closes), and
//!   asks for frames only while it moves.
//! - Cost: a closing list shows its last rows for 0.15 s; a choice made by
//!   the keyboard closes it the same way.

use crate::slider::motion::{DRAG_GLIDE, Tween};

/// How open a list over the composer is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reveal {
    tween: Tween,
}

impl Default for Reveal {
    fn default() -> Self {
        Self {
            tween: Tween::rest(0.0),
        }
    }
}

impl Reveal {
    /// The list should be `open` (or not) at `now` (seconds); `instant`:
    /// animations are off, so it is there, or gone, at once.
    pub fn follow(&mut self, open: bool, now: f32, instant: bool) {
        let to = if open { 1.0 } else { 0.0 };
        if instant {
            self.tween = Tween::rest(to);
        } else {
            self.tween.glide(to, now, DRAG_GLIDE);
        }
    }

    /// How open it is at `now`, from 0 (just appearing) to 1; `None` once
    /// it is gone (nothing to draw).
    #[must_use]
    pub fn shown(&self, now: f32) -> Option<f32> {
        let open = self.tween.at(now);
        (open > 0.0 || self.tween.moving(now)).then_some(open)
    }

    /// Whether it still moves at `now` (the screen asks for a frame).
    #[must_use]
    pub fn moving(&self, now: f32) -> bool {
        self.tween.moving(now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_opens_over_the_glide_and_then_rests() {
        let mut reveal = Reveal::default();
        assert_eq!(reveal.shown(0.0), None, "closed: nothing drawn");
        reveal.follow(true, 1.0, false);
        assert_eq!(reveal.shown(1.0), Some(0.0), "drawn from its first frame");
        assert!(reveal.moving(1.05));
        let midway = reveal.shown(1.05).unwrap();
        assert!(midway > 0.0 && midway < 1.0, "{midway}");
        assert_eq!(reveal.shown(1.2), Some(1.0));
        assert!(!reveal.moving(1.2), "no frames once open");
    }

    #[test]
    fn a_closing_list_is_drawn_until_it_is_gone() {
        let mut reveal = Reveal::default();
        reveal.follow(true, 0.0, false);
        reveal.follow(false, 1.0, false);
        assert!(reveal.shown(1.05).is_some_and(|open| open < 1.0));
        assert_eq!(reveal.shown(1.2), None);
        assert!(!reveal.moving(1.2));
    }

    #[test]
    fn reopened_while_closing_it_turns_back_from_where_it_is() {
        let mut reveal = Reveal::default();
        reveal.follow(true, 0.0, false);
        reveal.follow(false, 1.0, false);
        let at = reveal.shown(1.05).unwrap();
        reveal.follow(true, 1.05, false);
        let next = reveal.shown(1.05).unwrap();
        assert!((next - at).abs() < 1e-4, "{next} from {at}");
        assert_eq!(reveal.shown(1.3), Some(1.0));
    }

    #[test]
    fn with_animations_off_it_is_there_or_gone_at_once() {
        let mut reveal = Reveal::default();
        reveal.follow(true, 0.0, true);
        assert_eq!(reveal.shown(0.0), Some(1.0));
        assert!(!reveal.moving(0.0));
        reveal.follow(false, 0.0, true);
        assert_eq!(reveal.shown(0.0), None);
    }

    #[test]
    fn following_the_same_state_again_changes_nothing() {
        let mut reveal = Reveal::default();
        reveal.follow(true, 0.0, false);
        let before = reveal;
        reveal.follow(true, 0.05, false);
        assert_eq!(
            reveal, before,
            "a frame of the same menu does not restart it"
        );
    }
}
