//! What decorates the slider at its top stop and with Fast on, as the
//! Codex app draws them (the owner, 2026-11-14: "learn the colour change
//! at Ultra and with Fast, and the feeling of speeding up, as they are").
//!
//! [Decision Log]
//! - Purpose: the top stop's purple field revealed under the thumb, the
//!   burst when the thumb arrives there, the stops fading, the usage
//!   warning's shimmer; with Fast on, white dots streaming back from the
//!   thumb along the fill while the filled stops slide away.
//! - Constraint: the reference paints its field with a WebGL shader (a
//!   weighted mean of 12 moving colour blobs) blended over a gradient with
//!   `mix-blend-mode: luminosity`, and its warning with a gradient clipped
//!   to the text. GPUI has neither shaders, blend modes nor text clipped
//!   to a gradient.
//! - Choice: the same functions, evaluated on the CPU: [`field`] gives the
//!   blob colour at a point and time, [`luminosity`] is the W3C blend
//!   formula, [`backdrop`] the gradient under it, [`reveal`] the mask that
//!   sweeps it in; the kit samples them into a small image each frame.
//!   The warning's shimmer is a colour per character ([`shimmer`]).
//! - Differences, named: the field is sampled every 2 points (the
//!   reference's grain is that coarse; the GPU smooths the samples), so
//!   its grain is softer; the shimmer moves in steps of a character, not
//!   continuously across the glyphs; the blend is computed in sRGB as
//!   browsers do, so it matches the formula, not a captured frame.
#![expect(
    clippy::float_arithmetic,
    reason = "colour blending and animation curves are float formulas, kept readable against their sources"
)]

use super::motion::{BURST, Bezier, EASE, EASE_IN_OUT, EASE_OUT, SLIDE};

/// An sRGB colour, each channel 0 to 1.
pub type Rgb = [f32; 3];

/// The progress of a `length`-second animation `delay` seconds late, at
/// `elapsed` seconds, eased by `curve`; 0 before it starts.
fn progress(elapsed: f32, delay: f32, length: f32, curve: Bezier) -> f32 {
    if length <= 0.0 {
        return 1.0;
    }
    curve.at(((elapsed - delay) / length).clamp(0.0, 1.0))
}

fn mix(from: f32, to: f32, by: f32) -> f32 {
    from + (to - from) * by
}

/// `share` of `a` and the rest of `b`, as CSS `color-mix(in srgb …)`.
#[must_use]
pub fn mix_rgb(a: Rgb, b: Rgb, share: f32) -> Rgb {
    let [ar, ag, ab] = a;
    let [br, bg, bb] = b;
    [mix(br, ar, share), mix(bg, ag, share), mix(bb, ab, share)]
}

// --- The burst at the top stop ------------------------------------------

/// The burst's length, seconds.
pub const BURST_TIME: f32 = 0.62;
/// A burst particle's size.
pub const BURST_DOT: f32 = 5.0;

/// The 16 particles: where each ends (x, y, from the thumb's centre) and
/// its delay in seconds.
pub const BURST_PARTICLES: [(f32, f32, f32); 16] = [
    (-3.0, -34.0, 0.0),
    (15.0, -29.0, 0.008),
    (30.0, -19.0, 0.0),
    (34.0, -2.0, 0.012),
    (26.0, 20.0, 0.0),
    (12.0, 31.0, 0.010),
    (-6.0, 34.0, 0.0),
    (-22.0, 26.0, 0.014),
    (-32.0, 9.0, 0.0),
    (-32.0, -10.0, 0.008),
    (-21.0, -26.0, 0.0),
    (7.0, -24.0, 0.006),
    (24.0, -9.0, 0.0),
    (20.0, 10.0, 0.010),
    (-9.0, 21.0, 0.0),
    (-25.0, -5.0, 0.012),
];

/// The burst's peak, a share of its time.
const PEAK: f32 = 0.22;

/// A drawn dot: its centre's offset, scale and opacity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dot {
    /// Horizontal offset from its origin.
    pub x: f32,
    /// Vertical offset from its origin.
    pub y: f32,
    /// Its scale.
    pub scale: f32,
    /// Its opacity.
    pub opacity: f32,
}

/// How much of the blue particle `index` mixes in: the reference tints
/// every fourth particle from the third 18 % toward blue, and every
/// fourth from the fifth 32 %.
#[must_use]
pub const fn burst_blue(index: usize) -> f32 {
    match index % 4 {
        2 => 0.18,
        0 if index > 0 => 0.32,
        _ => 0.0,
    }
}

/// Burst particle `index` at `elapsed` seconds after the thumb arrived;
/// `None` once it is over. Keyframes: invisible at a quarter size, fully
/// seen at 1.28 at 22 %, then out to its place at 0.55, fading; the
/// curve applies to each stretch between keyframes, as CSS does.
#[must_use]
pub fn burst(index: usize, elapsed: f32) -> Option<Dot> {
    let &(x, y, delay) = BURST_PARTICLES.get(index)?;
    let t = (elapsed - delay) / BURST_TIME;
    if t >= 1.0 {
        return None;
    }
    let t = t.max(0.0);
    Some(if t < PEAK {
        let p = BURST.at(t / PEAK);
        Dot {
            x: 0.0,
            y: 0.0,
            scale: mix(0.25, 1.28, p),
            opacity: p,
        }
    } else {
        let p = BURST.at((t - PEAK) / (1.0 - PEAK));
        Dot {
            x: x * p,
            y: y * p,
            scale: mix(1.28, 0.55, p),
            opacity: 1.0 - p,
        }
    })
}

// --- Fast: dots streaming back along the fill ---------------------------

/// How many dots.
pub const FAST_DOTS: usize = 14;
/// One pass's length, seconds, before each dot's own spread.
pub const FAST_PASS: f32 = 1.9;
/// Each dot's speed varies by up to this share either way.
const FAST_SPREAD: f32 = 0.2;
/// A fast dot's size.
pub const FAST_DOT: f32 = 3.0;

/// The reference's per-dot pseudo-random number, 0 to 1 (the common
/// shader hash), for dot `index` and stream `salt`.
#[must_use]
pub fn hash(index: usize, salt: f32) -> f32 {
    let n = ((super::float(index) + 1.0) * 12.989_8 + salt * 78.233).sin() * 43_758.547;
    n - n.floor()
}

/// Fast dot `index`, `elapsed` seconds after the dots started with the
/// thumb `start` of the way along: how far back from the thumb it is, as
/// a share of the track's width (it starts at the thumb and streams
/// toward the start), its height as a share of the track's, its scale and
/// opacity. The pass fades in over its first 8 % and out over its last.
#[must_use]
pub fn fast_dot(index: usize, elapsed: f32, start: f32) -> Dot {
    let speed = 1.0 + (hash(index, 21.0) - 0.5) * 2.0 * FAST_SPREAD;
    let pass = FAST_PASS / speed;
    let stagger = FAST_PASS / super::float(FAST_DOTS);
    let behind = pass * (1.0 - start.clamp(0.0, 1.0));
    let delay = super::float(index) * stagger - behind;
    let t = ((elapsed - delay) / pass).rem_euclid(1.0);
    let fade = if t < 0.08 {
        t / 0.08
    } else if t > 0.92 {
        (1.0 - t) / 0.08
    } else {
        1.0
    };
    Dot {
        x: t,
        y: 0.12 + hash(index, 23.0) * 0.76,
        scale: 0.5 + hash(index, 12.0) * 0.45,
        opacity: (0.4 + hash(index, 11.0) * 0.6) * fade,
    }
}

// --- The stops ------------------------------------------------------------

/// How Fast stands for the stops' animation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FastStops {
    /// Off, settled.
    Off,
    /// Turned on `since` seconds ago (the filled stops slide away).
    On {
        /// Seconds since it was turned on.
        since: f32,
    },
    /// Turned off `since` seconds ago (they come back).
    Back {
        /// Seconds since it was turned off.
        since: f32,
    },
}

/// A stop's look: its offset, scale and opacity.
#[must_use]
pub fn stop(filled: bool, top: Option<f32>, fast: FastStops) -> Dot {
    let rest = Dot {
        x: 0.0,
        y: 0.0,
        scale: 1.0,
        opacity: 1.0,
    };
    match fast {
        FastStops::On { since } if filled => Dot {
            x: -85.0 * progress(since, 0.05, 1.15, EASE_IN_OUT),
            y: 0.0,
            scale: mix(1.0, 0.75, progress(since, 0.0, 0.8, SLIDE)),
            opacity: 1.0 - progress(since, 0.7, 0.5, EASE_IN_OUT),
        },
        FastStops::Back { since } if filled && top.is_none() => Dot {
            x: mix(10.0, 0.0, progress(since, 0.0, 0.35, EASE_OUT)),
            y: 0.0,
            scale: mix(0.75, 1.0, progress(since, 0.0, 0.35, EASE_IN_OUT)),
            opacity: progress(since, 0.0, 0.2, EASE_IN_OUT),
        },
        FastStops::On { .. } | FastStops::Back { .. } | FastStops::Off => match top {
            // At the top stop all stops fade (0.4 s after 0.1 s) and
            // shrink (0.4 s); leaving it they return in 0.2 s and 0.12 s.
            Some(since) => Dot {
                x: 0.0,
                y: 0.0,
                scale: mix(1.0, 0.75, progress(since, 0.0, 0.4, EASE)),
                opacity: 1.0 - progress(since, 0.1, 0.4, EASE_IN_OUT),
            },
            None => rest,
        },
    }
}

/// A stop's look leaving the top stop `since` seconds ago.
#[must_use]
pub fn stop_leaving_top(since: f32) -> Dot {
    Dot {
        x: 0.0,
        y: 0.0,
        scale: mix(0.75, 1.0, progress(since, 0.0, 0.12, EASE)),
        opacity: progress(since, 0.0, 0.2, EASE_IN_OUT),
    }
}

// --- The top stop's field -------------------------------------------------

/// The field's 12 colours.
const COLOURS: [Rgb; 12] = [
    [0.592, 0.388, 0.945],
    [0.831, 0.710, 0.953],
    [0.286, 0.000, 0.404],
    [0.145, 0.055, 0.478],
    [0.592, 0.000, 0.996],
    [0.780, 0.459, 0.914],
    [0.725, 0.576, 1.000],
    [0.400, 0.212, 0.820],
    [0.882, 0.690, 1.000],
    [0.498, 0.345, 0.957],
    [0.659, 0.275, 0.910],
    [0.212, 0.063, 0.400],
];

/// One blob's path: centre (x, y), reach (x, y), frequencies (x, y), and
/// whether x follows a sine (else a cosine; y the other one); then its
/// weight's pulse frequency and whether that is a sine.
type Blob = ((f32, f32), (f32, f32), (f32, f32), bool, f32, bool);

const BLOBS: [Blob; 12] = [
    ((0.18, 0.36), (0.18, 0.42), (0.42, 0.50), true, 0.91, true),
    ((0.34, 0.62), (0.24, 0.38), (0.62, 0.47), false, 1.07, false),
    ((0.52, 0.30), (0.28, 0.36), (0.38, 0.58), true, 0.76, true),
    ((0.70, 0.68), (0.24, 0.36), (0.54, 0.41), false, 1.18, false),
    ((0.88, 0.36), (0.16, 0.40), (0.74, 0.64), true, 1.03, true),
    ((0.12, 0.72), (0.20, 0.30), (0.48, 0.70), false, 0.83, false),
    ((0.30, 0.44), (0.22, 0.42), (0.58, 0.52), true, 1.24, true),
    ((0.46, 0.72), (0.26, 0.32), (0.68, 0.56), false, 0.96, false),
    ((0.60, 0.26), (0.28, 0.38), (0.44, 0.60), true, 1.11, true),
    ((0.76, 0.54), (0.22, 0.40), (0.50, 0.66), false, 0.72, false),
    ((0.92, 0.66), (0.15, 0.30), (0.70, 0.46), true, 1.29, true),
    ((0.06, 0.32), (0.14, 0.40), (0.40, 0.60), false, 0.88, false),
];

/// The field's pace.
const PACE: f32 = 1.25;
/// The field's horizontal stretch.
const STRETCH: f32 = 1.55;
/// How far the field's colour is held toward its darkest blob.
const HOLD: f32 = 0.96;
/// The grain's strength.
const GRAIN: f32 = 0.012;

/// The field at one moment: each blob's centre and weight, worked out
/// once per frame (a frame samples it a thousand times or more).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field {
    blobs: [((f32, f32), f32); 12],
}

impl Field {
    /// The field `time` seconds in.
    #[must_use]
    pub fn at(time: f32) -> Self {
        let blobs = BLOBS.map(
            |((cx, cy), (rx, ry), (fx, fy), sine_x, pulse, sine_pulse)| {
                let (wave_x, wave_y) = if sine_x {
                    ((time * PACE * fx).sin(), (time * PACE * fy).cos())
                } else {
                    ((time * PACE * fx).cos(), (time * PACE * fy).sin())
                };
                let beat = if sine_pulse {
                    (time * pulse).sin()
                } else {
                    (time * pulse).cos()
                };
                (
                    ((cx + wave_x * rx) * STRETCH, cy + wave_y * ry),
                    0.7 + 0.3 * beat,
                )
            },
        );
        Self { blobs }
    }

    /// The colour at `u` across the fill (0 at its start, 1 at its full
    /// width) and `v` up it (0 at the bottom); `grain` a number from -1
    /// to 1 for this sample.
    #[must_use]
    pub fn colour(&self, u: f32, v: f32, grain: f32) -> Rgb {
        let point = (u * STRETCH, 0.40 + v * 0.18);
        let mut sum = [0.0_f32; 3];
        let mut total = 0.0_f32;
        for (((cx, cy), pulse), colour) in self.blobs.iter().zip(COLOURS) {
            let (dx, dy) = (point.0 - cx, point.1 - cy);
            let weight = (-(dx * dx + dy * dy) * 12.0).exp() * pulse;
            total += weight;
            for (channel, value) in sum.iter_mut().zip(colour) {
                *channel += value * weight;
            }
        }
        let total = total.max(1e-4);
        let mean = sum.map(|channel| channel / total);
        let held = mix_rgb(mean, COLOURS[3], HOLD);
        held.map(|channel| channel.clamp(0.0, 1.0).powf(0.9) + grain * GRAIN)
    }
}

/// The field's colour at `u`, `v`, `time` seconds in ([`Field`]).
#[must_use]
pub fn field(u: f32, v: f32, time: f32, grain: f32) -> Rgb {
    Field::at(time).colour(u, v, grain)
}

/// The gradient under the field at `x` (0 to 1 across the fill): blue
/// mixed 60 % into the purple, the purple at 55 %, the purple mixed 72 %
/// with the blue at the end.
#[must_use]
pub fn backdrop(x: f32, blue: Rgb, purple: Rgb) -> Rgb {
    let start = mix_rgb(blue, purple, 0.6);
    let end = mix_rgb(purple, blue, 0.72);
    if x < 0.55 {
        mix_rgb(purple, start, (x / 0.55).clamp(0.0, 1.0))
    } else {
        mix_rgb(end, purple, ((x - 0.55) / 0.45).clamp(0.0, 1.0))
    }
}

fn lum([r, g, b]: Rgb) -> f32 {
    0.3 * r + 0.59 * g + 0.11 * b
}

/// The W3C `luminosity` blend: `backdrop`'s hue and saturation with
/// `source`'s luminosity.
#[must_use]
pub fn luminosity(backdrop: Rgb, source: Rgb) -> Rgb {
    let shift = lum(source) - lum(backdrop);
    let moved = backdrop.map(|channel| channel + shift);
    let light = lum(moved);
    let low = moved.iter().copied().fold(f32::INFINITY, f32::min);
    let high = moved.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    moved.map(|channel| {
        let mut channel = channel;
        if low < 0.0 {
            channel = light + (channel - light) * light / (light - low);
        }
        if high > 1.0 {
            channel = light + (channel - light) * (1.0 - light) / (high - light);
        }
        channel.clamp(0.0, 1.0)
    })
}

/// The field's reveal, seconds.
pub const REVEAL_TIME: f32 = 2.0;
/// The reveal's soft edge, points.
const REVEAL_EDGE: f32 = 120.0;

/// How much of the field shows at `x` points across a fill `width` wide,
/// `elapsed` seconds after the thumb reached the top stop: an edge
/// sweeping from the thumb's end to the start over 2 s.
#[must_use]
pub fn reveal(x: f32, width: f32, elapsed: f32) -> f32 {
    let edge = width * (1.0 - EASE.at((elapsed / REVEAL_TIME).clamp(0.0, 1.0)));
    ((x - (edge - REVEAL_EDGE)) / REVEAL_EDGE).clamp(0.0, 1.0)
}

// --- The usage warning ----------------------------------------------------

/// The warning's shimmer, seconds.
pub const SHIMMER_TIME: f32 = 1.1;

/// How much lighter (toward the text colour) the warning is at `at` of
/// its width, `elapsed` seconds after it showed: a light band crossing it
/// once from right to left, then the purple alone.
#[must_use]
pub fn shimmer(at: f32, elapsed: f32) -> f32 {
    if elapsed >= SHIMMER_TIME {
        return 0.0;
    }
    let position = 1.0 - EASE_OUT.at((elapsed / SHIMMER_TIME).clamp(0.0, 1.0));
    let g = (at + 2.0 * position) / 3.0;
    let ramp =
        |from: f32, to: f32, a: f32, b: f32| mix(a, b, ((g - from) / (to - from)).clamp(0.0, 1.0));
    if g <= 0.42 || g >= 0.58 {
        0.0
    } else if g < 0.47 {
        ramp(0.42, 0.47, 0.0, 0.3)
    } else if g < 0.50 {
        ramp(0.47, 0.50, 0.3, 0.6)
    } else if g < 0.53 {
        ramp(0.50, 0.53, 0.6, 0.3)
    } else {
        ramp(0.53, 0.58, 0.3, 0.0)
    }
}
