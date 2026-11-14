//! How the drawing moves from the new session to the open session, and
//! how the star becomes the agent's avatar: one bounded transition, then
//! still.
//!
//! [Decision Log]
//! - Purpose: the owner wants the first message to flow on smoothly: the
//!   star goes to the head of the agent's reply, turns into a circle, and
//!   the agent's own mark appears inside (2026-11-14 ~05:55 UTC), with
//!   "short, quick, fine" motion everywhere and a way to turn it off.
//! - Constraints: no continuous animation; the journey advances once per
//!   presented frame while it runs (the app asks for the next frame, no
//!   timer) and holds still with reduced motion, a software renderer or
//!   animations off (`kit::motion::held`), where the end state shows at
//!   once.
//! - Choice: three phases on one clock, each eased by the Codex app's
//!   curve (`cubic-bezier(.23, 1, .32, 1)`): the move ([`MOVE`]: the V
//!   opens into the frame, the star travels and shrinks), the morph
//!   ([`MORPH`]: star to circle), the mark ([`MARK`]: the circle becomes a
//!   ring and the agent's mark fades in). The durations restate the kit's
//!   motion tokens (`STANDARD` 300 ms, `QUICK` 150 ms; `view` cannot read
//!   the kit), through the slider's constants; an app test pins them.
//! - Cost: about eighteen frames once per first message, nothing after.

use super::{Arms, Figure, Loops, Pt, Star};
use crate::slider::motion::{DRAG_GLIDE, GLIDE, SLIDE};

/// The move: seconds.
pub const MOVE: f32 = GLIDE;
/// Star to circle: seconds.
pub const MORPH: f32 = DRAG_GLIDE;
/// Circle to ring with the agent's mark: seconds.
pub const MARK: f32 = DRAG_GLIDE;

/// How far each phase has come, eased, 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Journey {
    /// The move.
    pub moved: f32,
    /// Star to circle.
    pub morphed: f32,
    /// The ring and the agent's mark.
    pub marked: f32,
}

impl Journey {
    /// The end: everything in place.
    pub const DONE: Self = Self {
        moved: 1.0,
        morphed: 1.0,
        marked: 1.0,
    };
}

/// The journey `elapsed` seconds after the first message was sent; `still`
/// (reduced motion, a software renderer, animations off) is the end at
/// once.
#[must_use]
pub fn journey(elapsed: f32, still: bool) -> Journey {
    if still {
        return Journey::DONE;
    }
    #[expect(
        clippy::float_arithmetic,
        reason = "a phase's share of its own duration"
    )]
    let phase = |start: f32, length: f32| SLIDE.at(((elapsed - start) / length).clamp(0.0, 1.0));
    #[expect(clippy::float_arithmetic, reason = "the phases follow one another")]
    let (morph_start, mark_start) = (MOVE, MOVE + MORPH);
    Journey {
        moved: phase(0.0, MOVE),
        morphed: phase(morph_start, MORPH),
        marked: phase(mark_start, MARK),
    }
}

/// Whether the journey has ended `elapsed` seconds in (no more frames).
#[must_use]
pub fn ended(elapsed: f32) -> bool {
    #[expect(clippy::float_arithmetic, reason = "the three phases' total")]
    let total = MOVE + MORPH + MARK;
    elapsed >= total
}

#[expect(clippy::float_arithmetic, reason = "linear interpolation")]
fn mix(a: f32, b: f32, t: f32) -> f32 {
    (b - a).mul_add(t, a)
}

fn mix_pt(a: Pt, b: Pt, t: f32) -> Pt {
    Pt {
        x: mix(a.x, b.x, t),
        y: mix(a.y, b.y, t),
    }
}

/// The V `t` of the way from `from` to `to`; a loop only one side has
/// fades in or out, and so does a V only one side has.
fn mix_loops(from: Option<Loops>, to: Option<Loops>, t: f32) -> Option<Loops> {
    match (from, to) {
        (Some(a), Some(b)) => {
            let loops = a.loops.max(b.loops);
            let outer = match a.loops.cmp(&b.loops) {
                std::cmp::Ordering::Greater => mix(a.outer, 0.0, t),
                std::cmp::Ordering::Less => mix(0.0, b.outer, t),
                std::cmp::Ordering::Equal => mix(a.outer, b.outer, t),
            };
            Some(Loops {
                arms: Arms {
                    left: mix_pt(a.arms.left, b.arms.left, t),
                    apex: mix_pt(a.arms.apex, b.arms.apex, t),
                    right: mix_pt(a.arms.right, b.arms.right, t),
                },
                pitch: mix(a.pitch, b.pitch, t),
                loops,
                outer,
                alpha: mix(a.alpha, b.alpha, t),
            })
        }
        (Some(a), None) => Some(Loops {
            alpha: mix(a.alpha, 0.0, t),
            ..a
        }),
        (None, Some(b)) => Some(Loops {
            alpha: mix(0.0, b.alpha, t),
            ..b
        }),
        (None, None) => None,
    }
}

/// The backdrop `journey.moved` of the way from the new session's figure
/// to the open session's, with the star on its way to `avatar` (the head
/// of the agent's coming reply). Once the move ends the star is the
/// avatar's to draw, so the backdrop has none.
#[must_use]
pub fn between(from: &Figure, to: &Figure, avatar: Star, journey: Journey) -> Figure {
    let t = journey.moved;
    let star = from.star.and_then(|star| {
        (t < 1.0).then(|| Star {
            centre: mix_pt(star.centre, avatar.centre, t),
            span: mix(star.span, avatar.span, t),
        })
    });
    Figure {
        v: mix_loops(from.v, to.v, t),
        star,
    }
}
