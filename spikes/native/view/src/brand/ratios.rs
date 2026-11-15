//! The mark's measured ratios (`docs/design/brand/README.md`, "Multiline
//! rules"): every drawing derived from the mark keeps these and changes
//! only its pitch.
#![expect(
    clippy::float_arithmetic,
    reason = "ratios of the mark's measured lengths, kept as the fractions they were measured as"
)]

/// The pitch the mark was measured at (view-box units): line + gap.
const MARK_PITCH: f32 = 44.4;

/// A line's weight as a share of the pitch (19.0 of 44.4).
pub const LINE: f32 = 19.0 / MARK_PITCH;

/// A gap as a share of the pitch (25.4 of 44.4; 1.33 lines).
pub const GAP: f32 = 25.4 / MARK_PITCH;

/// The arms' angle from the horizontal, in degrees (the mark's path steps
/// are 1 : √3).
pub const ARM_DEGREES: f32 = 60.0;

/// The star's tip-to-tip span as a share of the V's outer width.
pub const STAR_OF_WIDTH: f32 = 0.33;

/// How far down the V's height the star's centre sits, as a share.
pub const STAR_DOWN: f32 = 0.23;

/// The star's tip radius in the mark's units (half its 490.2 span).
pub const STAR_TIP: f32 = 245.1;

/// How far the open session's frame leans in from the vertical at most, in
/// degrees: the owner's sketch of the open session (2026-11-14, about
/// 1950 wide) draws its arms 18–20° from the vertical, ending in the side
/// margins short of the conversation.
pub const FRAME_LEAN_DEGREES: f32 = 20.0;

/// The air between the drawing and any text or control, in points.
pub const CLEARANCE: f32 = 12.0;

/// The pitch for a content area `width` points wide: about a hundred and
/// forty-fourth of it, kept between 6 and 8 points (7 at a 1280 window, 8
/// at 1920, 6 on a phone), so a line is 2.6 to 3.4 points and a gap 3.4 to
/// 4.6: separate lines at scale 1, never a grey band.
#[must_use]
pub fn pitch_for(width: f32) -> f32 {
    (width / 144.0).round().clamp(6.0, 8.0)
}

/// Half the width of a band of `loops` nested loops (2 × `loops` lines)
/// at `pitch`, from its centre line to its outer edge.
#[must_use]
pub fn half_band(loops: u8, pitch: f32) -> f32 {
    (f32::from(loops) - 0.5).mul_add(pitch, LINE * pitch / 2.0)
}
