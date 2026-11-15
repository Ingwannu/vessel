//! The mark's construction as geometry: lines offset from a centre line,
//! turning about shared centres (so every turn is concentric, as in the
//! mark), closed into loops, and the owner's star as an outline that can
//! relax into a circle.
#![expect(
    clippy::float_arithmetic,
    reason = "offset curves, arcs and a cubic are float formulas; written as formulas they stay checkable against the mark's measurements"
)]

use super::{Pt, ratios};

/// The step, in radians, between two samples of an arc: fine enough that
/// the chords stay under a tenth of a device pixel at the largest radius
/// drawn (a 64-point loop at scale 2).
const ARC_STEP: f32 = 0.035;

/// The unit vector from `a` to `b` (zero when they meet).
fn unit(a: Pt, b: Pt) -> Pt {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx.hypot(dy);
    if length <= f32::EPSILON {
        return Pt { x: 0.0, y: 0.0 };
    }
    Pt {
        x: dx / length,
        y: dy / length,
    }
}

/// The left-hand normal of a direction (in the screen's y-down plane).
const fn normal(u: Pt) -> Pt {
    Pt { x: -u.y, y: u.x }
}

/// `p` moved by `d` along `v`.
fn along(p: Pt, v: Pt, d: f32) -> Pt {
    Pt {
        x: v.x.mul_add(d, p.x),
        y: v.y.mul_add(d, p.y),
    }
}

/// Samples of an arc about `centre`, radius `r`, from angle `from` through
/// `sweep` radians, both ends included.
fn arc(centre: Pt, r: f32, from: f32, sweep: f32, out: &mut Vec<Pt>) {
    let steps = (sweep.abs() / ARC_STEP).ceil().clamp(2.0, 400.0);
    let count = whole(steps);
    for i in 0..=count {
        let a = sweep.mul_add(f32::from(i) / steps, from);
        out.push(Pt {
            x: r.mul_add(a.cos(), centre.x),
            y: r.mul_add(a.sin(), centre.y),
        });
    }
}

/// A small non-negative float as a count (clamped to what an arc needs).
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to 2..=400 by the caller; `as` saturates"
)]
fn whole(steps: f32) -> u16 {
    steps as u16
}

/// The line at signed distance `d` from the centre line `pts`, whose
/// inner vertices turn on circles of centre-line radius `radius`: every
/// offset turns about the same centre, so the lines stay one pitch apart
/// through the turn (the mark's concentric turns).
#[must_use]
pub fn offset(pts: &[Pt], radius: f32, d: f32) -> Vec<Pt> {
    let mut out = Vec::new();
    let dirs: Vec<Pt> = pts
        .windows(2)
        .filter_map(|w| match w {
            [a, b] => Some(unit(*a, *b)),
            _ => None,
        })
        .collect();
    let (Some(first), Some(last), Some(&start), Some(&end)) =
        (dirs.first(), dirs.last(), pts.first(), pts.last())
    else {
        return out;
    };
    out.push(along(start, normal(*first), d));
    for (pair, &p) in dirs.windows(2).zip(pts.iter().skip(1)) {
        let [u1, u2] = pair else { continue };
        let theta = (u1.x * u2.x + u1.y * u2.y).clamp(-1.0, 1.0).acos();
        if theta < 1e-5 {
            continue;
        }
        let tangent = radius * (theta / 2.0).tan();
        let inward = unit(
            Pt { x: 0.0, y: 0.0 },
            Pt {
                x: u2.x - u1.x,
                y: u2.y - u1.y,
            },
        );
        let centre = along(p, inward, radius / (theta / 2.0).cos());
        let (n1, n2) = (normal(*u1), normal(*u2));
        let inside = n1.x * inward.x + n1.y * inward.y > 0.0;
        let r = if inside { radius - d } else { radius + d };
        let a = along(along(p, *u1, -tangent), n1, d);
        let b = along(along(p, *u2, tangent), n2, d);
        let from = (a.y - centre.y).atan2(a.x - centre.x);
        let to = (b.y - centre.y).atan2(b.x - centre.x);
        let turning_right = u1.x * u2.y - u1.y * u2.x > 0.0;
        let mut sweep = to - from;
        if turning_right {
            while sweep < 0.0 {
                sweep += std::f32::consts::TAU;
            }
        } else {
            while sweep > 0.0 {
                sweep -= std::f32::consts::TAU;
            }
        }
        arc(centre, r, from, sweep, &mut out);
    }
    out.push(along(end, normal(*last), d));
    out
}

/// The closed loop at distance `d` around the open centre line: out along
/// one side, a half circle about the end, back along the other side, a
/// half circle about the start (the mark's capsule; at `d = pitch / 2`
/// its hole is exactly one gap).
#[must_use]
pub fn loop_around(pts: &[Pt], radius: f32, d: f32) -> Vec<Pt> {
    let (Some(&first), Some(&second), Some(&before_last), Some(&last)) =
        (pts.first(), pts.get(1), pts.iter().rev().nth(1), pts.last())
    else {
        return Vec::new();
    };
    let mut out = offset(pts, radius, d);
    let n_end = normal(unit(before_last, last));
    arc(
        last,
        d,
        n_end.y.atan2(n_end.x),
        -std::f32::consts::PI,
        &mut out,
    );
    let mut back = offset(pts, radius, -d);
    back.reverse();
    out.extend(back);
    let n_start = normal(unit(first, second));
    arc(
        first,
        d,
        (-n_start.y).atan2(-n_start.x),
        -std::f32::consts::PI,
        &mut out,
    );
    out
}

/// The owner's star (the mark's second path), as points around its
/// centre for a tip-to-tip `span`, relaxed toward the circle through its
/// tips by `round` (0: the star, 1: the circle). The quadrant is the
/// mark's own: a straight flank from the tip, a cubic to the waist, a
/// straight flank to the next tip; the other three are its rotations.
#[must_use]
pub fn star(centre: Pt, span: f32, round: f32) -> Vec<Pt> {
    // The mark's numbers, relative to the star's centre (1000, 714.75).
    const TIP: Pt = Pt { x: 0.0, y: -245.1 };
    const FLANK: Pt = Pt {
        x: 24.73,
        y: -139.46,
    };
    const C1: Pt = Pt {
        x: 38.37,
        y: -81.19,
    };
    const C2: Pt = Pt {
        x: 81.04,
        y: -38.53,
    };
    const WAIST: Pt = Pt {
        x: 139.3,
        y: -24.89,
    };
    const SIDE: Pt = Pt {
        x: 244.94,
        y: -0.15,
    };
    const STEPS: u16 = 24;
    let mut quadrant = Vec::new();
    let line = |from: Pt, to: Pt, out: &mut Vec<Pt>| {
        for i in 0..STEPS {
            let share = f32::from(i) / f32::from(STEPS);
            out.push(Pt {
                x: (to.x - from.x).mul_add(share, from.x),
                y: (to.y - from.y).mul_add(share, from.y),
            });
        }
    };
    line(TIP, FLANK, &mut quadrant);
    for i in 0..STEPS {
        let along_curve = f32::from(i) / f32::from(STEPS);
        let rest = 1.0 - along_curve;
        let weights = [
            rest * rest * rest,
            3.0 * rest * rest * along_curve,
            3.0 * rest * along_curve * along_curve,
            along_curve * along_curve * along_curve,
        ];
        let [w0, w1, w2, w3] = weights;
        quadrant.push(Pt {
            x: w0 * FLANK.x + w1 * C1.x + w2 * C2.x + w3 * WAIST.x,
            y: w0 * FLANK.y + w1 * C1.y + w2 * C2.y + w3 * WAIST.y,
        });
    }
    line(WAIST, SIDE, &mut quadrant);
    let scale = span / (2.0 * ratios::STAR_TIP);
    let round = round.clamp(0.0, 1.0);
    let mut out = Vec::with_capacity(quadrant.len().saturating_mul(4));
    for turn in 0..4_u8 {
        let (sin, cos) = (std::f32::consts::FRAC_PI_2 * f32::from(turn)).sin_cos();
        for q in &quadrant {
            let (x, y) = (q.x.mul_add(cos, -q.y * sin), q.x.mul_add(sin, q.y * cos));
            let r = x.hypot(y);
            let toward = (ratios::STAR_TIP - r).mul_add(round, r) / r.max(f32::EPSILON);
            out.push(Pt {
                x: (x * toward).mul_add(scale, centre.x),
                y: (y * toward).mul_add(scale, centre.y),
            });
        }
    }
    out
}
