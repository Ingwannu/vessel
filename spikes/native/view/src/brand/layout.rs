//! Where each screen's drawing goes, from what the screen measured, and
//! how it degrades as the window narrows (decided in the study over the
//! real screens at 390, 1280 and 1920 wide).
//!
//! - New session, wide: the whole V, its apex under the pickers, its arms
//!   clearing the heading and the pickers by [`ratios::CLEARANCE`], the
//!   star between the arms above the composer.
//! - New session, narrow (the arms cannot clear the heading or the
//!   pickers): the star alone in the space above the heading and the V's
//!   bottom under the pickers.
//! - Open session: the V grown into a frame in the side margins, its apex
//!   below the screen; as many loops as the margin holds after the air
//!   kept beside the column (4, 3, 2), none under two; its arms lean in as
//!   far as the margin allows and no further than
//!   [`ratios::FRAME_LEAN_DEGREES`], so on a wide window they end in the
//!   margin as in the owner's sketch, not at the composer's corners.
#![expect(
    clippy::float_arithmetic,
    reason = "placing lines against measured boxes is float geometry; each formula mirrors the study's drawing"
)]

use super::{Area, Arms, Figure, Loops, Pt, Star, ratios};

/// The space kept at the content area's top and sides, in points.
const MARGIN: f32 = 16.0;
/// How far below a frame's apex is placed when its arms are parallel.
const FAR: f32 = 100_000.0;

/// What the new-session screen measured, in the content area's own
/// coordinates or the window's (all four alike).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NewSession {
    /// The area the drawing may fill (the content panel).
    pub content: Area,
    /// The composer's card (opaque: the V passes behind it).
    pub composer: Area,
    /// The heading's text box ("What should we work on?").
    pub heading: Area,
    /// The pickers under the composer (machine, folder).
    pub pickers: Area,
}

/// What the open session measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Session {
    /// The conversation's area (under the session's bar).
    pub content: Area,
    /// The conversation column's left edge.
    pub column_left: f32,
    /// The conversation column's right edge.
    pub column_right: f32,
    /// The air kept between the frame and the column (the agent's avatar
    /// sits inside the column, at the head of its reply).
    pub air: f32,
}

fn tan_arm() -> f32 {
    ratios::ARM_DEGREES.to_radians().tan()
}

fn sin_arm() -> f32 {
    ratios::ARM_DEGREES.to_radians().sin()
}

/// The new session's drawing.
#[must_use]
pub fn new_session(screen: &NewSession) -> Figure {
    wide(screen).unwrap_or_else(|| narrow(screen))
}

fn wide(screen: &NewSession) -> Option<Figure> {
    let NewSession {
        content,
        composer,
        heading,
        pickers,
    } = *screen;
    let loops = 4;
    let pitch = ratios::pitch_for(content.w);
    let half = ratios::half_band(loops, pitch);
    let across = half / sin_arm();
    let centre = composer.x + composer.w / 2.0;
    // The left arm moves left as it rises: each box's top-right corner,
    // grown by the clearance, bounds how low the apex may sit.
    let clear = |box_: Area| {
        (box_.y - ratios::CLEARANCE)
            + tan_arm() * (centre - (box_.right() + ratios::CLEARANCE) - across)
    };
    let apex_y = clear(heading)
        .min(clear(pickers))
        .min(content.bottom() - 30.0 - 2.0 * half);
    if apex_y <= composer.bottom() + 2.0 * half + 60.0 {
        return None;
    }
    let mut top_y = content.y + 22.0 + half;
    let mut top_x = centre - (apex_y - top_y) / tan_arm();
    if top_x - half < content.x + MARGIN {
        top_x = content.x + MARGIN + half;
        top_y = apex_y - (centre - top_x) * tan_arm();
    }
    let arms = Arms {
        left: Pt { x: top_x, y: top_y },
        apex: Pt {
            x: centre,
            y: apex_y,
        },
        right: Pt {
            x: 2.0 * centre - top_x,
            y: top_y,
        },
    };
    let outer_width = 2.0 * (centre - top_x + across);
    let v_top = top_y - half;
    let star_y = ratios::STAR_DOWN.mul_add(apex_y - v_top + 40.0, v_top);
    let inside = 2.0 * ((apex_y - star_y) / tan_arm() - across);
    let span = (ratios::STAR_OF_WIDTH * outer_width)
        .min(0.75 * inside)
        .min(2.0 * (heading.y.min(composer.y) - ratios::CLEARANCE - star_y));
    Some(Figure {
        v: Some(Loops {
            arms,
            pitch,
            loops,
            outer: 1.0,
            alpha: 1.0,
        }),
        star: (span > 0.0).then_some(Star {
            centre: Pt {
                x: centre,
                y: star_y,
            },
            span,
        }),
    })
}

fn narrow(screen: &NewSession) -> Figure {
    let NewSession {
        content,
        composer,
        heading,
        pickers,
    } = *screen;
    let loops = 3;
    let pitch = ratios::pitch_for(content.w);
    let half = ratios::half_band(loops, pitch);
    let centre = composer.x + composer.w / 2.0;
    let (top, bottom) = (content.y + 20.0, heading.y - ratios::CLEARANCE);
    let span = (0.62 * (bottom - top)).min(0.45 * content.w);
    let star = (span >= 32.0).then_some(Star {
        centre: Pt {
            x: centre,
            y: f32::midpoint(top, bottom),
        },
        span,
    });
    let end_y = pickers.bottom() + ratios::CLEARANCE + half;
    let drop = content.bottom() - 24.0 - end_y - 2.0 * half;
    let v = (drop >= 40.0).then(|| {
        let reach = drop / tan_arm();
        Loops {
            arms: Arms {
                left: Pt {
                    x: centre - reach,
                    y: end_y,
                },
                apex: Pt {
                    x: centre,
                    y: end_y + drop,
                },
                right: Pt {
                    x: centre + reach,
                    y: end_y,
                },
            },
            pitch,
            loops,
            outer: 1.0,
            alpha: 1.0,
        }
    });
    Figure { v, star }
}

/// The open session's frame.
#[must_use]
pub fn session(screen: &Session) -> Figure {
    let Session {
        content,
        column_left,
        column_right,
        air,
    } = *screen;
    let pitch = ratios::pitch_for(content.w);
    let margin = ratios::CLEARANCE;
    let room = column_left - content.x - air - margin;
    let Some(loops) = [4_u8, 3, 2]
        .into_iter()
        .find(|&loops| 2.0 * ratios::half_band(loops, pitch) <= room)
    else {
        return Figure::default();
    };
    let half = ratios::half_band(loops, pitch);
    let centre = f32::midpoint(column_left, column_right);
    let top_x = content.x + margin + half;
    let top_y = content.y + 18.0 + half;
    let lean = ratios::FRAME_LEAN_DEGREES.to_radians().tan() * (content.bottom() - top_y);
    let bottom_x = top_x.max((column_left - air - half).min(top_x + lean));
    let apex_y = if bottom_x - top_x < 1.0 {
        top_y + FAR
    } else {
        let slope = (bottom_x - top_x) / (content.bottom() - top_y);
        (top_y + (centre - top_x) / slope).min(top_y + FAR)
    };
    Figure {
        v: Some(Loops {
            arms: Arms {
                left: Pt { x: top_x, y: top_y },
                apex: Pt {
                    x: centre,
                    y: apex_y,
                },
                right: Pt {
                    x: 2.0 * centre - top_x,
                    y: top_y,
                },
            },
            pitch,
            loops,
            outer: 1.0,
            alpha: 1.0,
        }),
        star: None,
    }
}
