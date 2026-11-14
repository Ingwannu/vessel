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
//! - The empty board: the star above its message and the V's bottom turn
//!   under it (the new session's narrow figure at a message's size; the
//!   coordinator's pick from the study of 2026-11-14).
//! - A card in the composer's place (the first run's pairing card, for the
//!   onboarding screens to mount): the new session's figure, the card as
//!   its composer and no text beside it.
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

/// What the empty board measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmptyBoard {
    /// The area the drawing may fill (the content panel).
    pub content: Area,
    /// The message block: its title, the line under it and the button.
    pub message: Area,
}

/// The star's span on the empty board: a little over a twentieth of the
/// content's width, between the sizes the study drew (44 and 56 points).
fn board_star(content_w: f32) -> f32 {
    (0.055 * content_w).clamp(44.0, 56.0)
}

/// The empty board's drawing: the star above the message and the V's
/// bottom turn (three loops) under it; each only where its room allows.
#[must_use]
pub fn empty_board(screen: &EmptyBoard) -> Figure {
    let EmptyBoard { content, message } = *screen;
    let centre = message.x + message.w / 2.0;
    let span = board_star(content.w);
    let star_y = message.y - 28.0 - span / 2.0;
    let star = (star_y - span / 2.0 >= content.y + MARGIN).then_some(Star {
        centre: Pt {
            x: centre,
            y: star_y,
        },
        span,
    });
    let loops = 3;
    let pitch = ratios::pitch_for(content.w);
    let half = ratios::half_band(loops, pitch);
    let top = message.bottom() + ratios::CLEARANCE + half;
    let drop = (content.bottom() - 24.0 - half - top).min(150.0);
    let v = (drop >= 40.0).then(|| {
        let reach = drop / tan_arm();
        Loops {
            arms: Arms {
                left: Pt {
                    x: centre - reach,
                    y: top,
                },
                apex: Pt {
                    x: centre,
                    y: top + drop,
                },
                right: Pt {
                    x: centre + reach,
                    y: top,
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

/// What a screen with one card in the composer's place measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardScreen {
    /// The area the drawing may fill.
    pub content: Area,
    /// The card (opaque: the V passes behind it).
    pub card: Area,
}

/// The new session's figure around a card that stands where the composer
/// does, with no heading above it and no pickers under it. Wide (the V's
/// arms start beside the card): the arms pass behind the card (it is
/// opaque and nothing beside it is text), the star above it. Narrow: the
/// star above the card and the V's bottom under it, as the new session's
/// narrow figure.
#[must_use]
pub fn card_screen(screen: &CardScreen) -> Figure {
    let CardScreen { content, card } = *screen;
    card_wide(screen).unwrap_or_else(|| {
        narrow(&NewSession {
            content,
            composer: card,
            heading: card,
            pickers: card,
        })
    })
}

fn card_wide(screen: &CardScreen) -> Option<Figure> {
    let CardScreen { content, card } = *screen;
    let loops = 4;
    let pitch = ratios::pitch_for(content.w);
    let half = ratios::half_band(loops, pitch);
    let centre = card.x + card.w / 2.0;
    let apex_y = (content.bottom() - 24.0 - half)
        .min(0.55_f32.mul_add(content.bottom() - card.bottom(), card.bottom()));
    let top_y = content.y + 22.0 + half;
    let top_x = (content.x + MARGIN + half).max(centre - (apex_y - top_y) / tan_arm());
    // the arms must start beside the card, or only their ends would show
    if top_x + half >= card.x || apex_y <= card.bottom() + half {
        return None;
    }
    let top_y = apex_y - (centre - top_x) * tan_arm();
    let outer = 2.0 * (centre - top_x + half / sin_arm());
    let span = (ratios::STAR_OF_WIDTH * outer).min(0.45 * (card.y - content.y));
    let star_y = card.y - ratios::CLEARANCE - 18.0 - span / 2.0;
    let star = (star_y - span / 2.0 >= content.y + MARGIN).then_some(Star {
        centre: Pt {
            x: centre,
            y: star_y,
        },
        span,
    });
    Some(Figure {
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
        star,
    })
}
