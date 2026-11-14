//! The brand drawn on screen: the mark's V grown large behind the new
//! session's composer, the same V framing an open session, the star that
//! travels from one to the other and becomes the agent's avatar (the
//! owner, 2026-11-14: "Multiline Lettering Multiline Logo", and the three
//! drawings that came with it).
//!
//! What a drawing *is* (where its lines run, how many, where the star
//! sits, how it moves between screens) is decided here without a UI
//! toolkit and tested; the kit rasterises the [`Shape`]s at the display's
//! own pixels.
//!
//! [Decision Log]
//! - Purpose: the owner wants the mark's multiline character large on the
//!   screen: the composer inside the mark's V with the star above it, the
//!   V framing the conversation, the lines read as parallel traces.
//! - Constraints: no line may sit under text at a contrast worse than
//!   today's; still by default (no cost when nothing moves); sharp at
//!   scale 1, 1.25, 1.5 and 2; works from 360 points wide.
//! - Alternatives (all drawn over the real screens in the study, dark and
//!   light, 390 to 1920 wide): the owner's mark scaled up untouched (its
//!   lines become 12 px bands and pass under the heading and the pickers,
//!   whose contrast fell from 4.58 to 3.07); the same lines as circuit
//!   traces with 45° turns and pads (kept for the window frame and the
//!   window controls).
//! - Choice: a derived drawing in the mark's own construction (nested
//!   loops on one pitch, turns concentric at (k + ½) pitches, 60° arms,
//!   the owner's star at a third of the V's width), its pitch chosen for
//!   the screen ([`ratios::pitch_for`]) and its arms placed to clear every
//!   text box by [`ratios::CLEARANCE`]; it degrades in steps as the window
//!   narrows ([`layout`]).
//! - Why: it reads as the mark at any size (the mark's ratios, not its
//!   pixels), keeps every line separate at scale 1, and never passes
//!   under text, so contrast is today's by construction.
//! - Cost: geometry is a few hundred points per figure, computed when the
//!   layout or the transition changes; the kit caches the raster.

pub mod geometry;
pub mod layout;
pub mod motion;
pub mod ratios;
mod shapes;
#[cfg(test)]
mod tests;

pub use shapes::{Ink, RING, Shape, avatar, shapes};

/// A point in logical points (the screen's y grows downward).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pt {
    /// Across.
    pub x: f32,
    /// Down.
    pub y: f32,
}

/// A rectangle in logical points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    /// The left edge.
    pub x: f32,
    /// The top edge.
    pub y: f32,
    /// The width.
    pub w: f32,
    /// The height.
    pub h: f32,
}

impl Area {
    /// The right edge.
    #[must_use]
    pub fn right(self) -> f32 {
        #[expect(clippy::float_arithmetic, reason = "an edge is origin plus size")]
        let right = self.x + self.w;
        right
    }

    /// The bottom edge.
    #[must_use]
    pub fn bottom(self) -> f32 {
        #[expect(clippy::float_arithmetic, reason = "an edge is origin plus size")]
        let bottom = self.y + self.h;
        bottom
    }

    /// The smallest area holding both: several boxes the drawing keeps
    /// clear of as one (the pickers and a note under them; the empty
    /// board's title, line and button).
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let (x, y) = (self.x.min(other.x), self.y.min(other.y));
        let (right, bottom) = (
            self.right().max(other.right()),
            self.bottom().max(other.bottom()),
        );
        #[expect(clippy::float_arithmetic, reason = "a size is edge minus origin")]
        let (w, h) = (right - x, bottom - y);
        Self { x, y, w, h }
    }
}

/// The V's centre line: up-left arm top, apex, up-right arm top.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arms {
    /// The left arm's free end.
    pub left: Pt,
    /// Where the arms meet (may lie far below the screen: a frame).
    pub apex: Pt,
    /// The right arm's free end.
    pub right: Pt,
}

/// Nested loops around the V's centre line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loops {
    /// The centre line.
    pub arms: Arms,
    /// Line plus gap, in points.
    pub pitch: f32,
    /// How many loops (twice as many lines across).
    pub loops: u8,
    /// The outermost loop's share of the ink, 0 to 1 (a loop added or
    /// dropped in a transition fades instead of popping).
    pub outer: f32,
    /// The whole V's share of the ink, 0 to 1 (a V that a screen does not
    /// have fades out in a transition).
    pub alpha: f32,
}

/// The owner's star.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Star {
    /// Its centre.
    pub centre: Pt,
    /// Tip to tip, in points.
    pub span: f32,
}

/// What one screen draws behind its content.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Figure {
    /// The V (whole, its bottom only, or grown into a frame), if any.
    pub v: Option<Loops>,
    /// The star, if the screen shows it.
    pub star: Option<Star>,
}
