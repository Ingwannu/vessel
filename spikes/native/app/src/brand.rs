//! The brand's drawing on the shell: the mark's V behind the new session's
//! composer, the same V grown into the open session's frame, the star that
//! travels between them on the first message and becomes the agent's
//! avatar (`view::brand` decides all of it; this module measures, holds
//! the journey's clock and hands shapes to the kit).
//!
//! [Decision Log]
//! - Purpose: one drawing that belongs to no screen: it must keep moving
//!   while the route changes from the new session to the session, so it is
//!   the shell's, painted in the content panel behind whichever screen is
//!   shown.
//! - Constraints: the screens are other tracks' files (the new session the
//!   lead's and P's, the session C's); each gives one listener to the
//!   element it already lays out, and nothing else changes in them.
//! - Choice: the screens report their boxes while they are laid out
//!   (`on_children_prepainted`, the prepaint phase); the backdrop, painted
//!   after the whole window's prepaint, makes the figure from those boxes
//!   in the same frame. A screen that reported nothing this frame draws
//!   nothing (the board, settings). The journey is a start time and the
//!   figure it starts from; it asks for the next presented frame while it
//!   runs (the display's own rate, the owner's word 42) and stops asking
//!   when it ends; under held motion (reduced, off, a software renderer)
//!   the end shows at once.
//! - Alternatives: each screen drawing its own figure (the two would not
//!   be one drawing across the route change); boxes computed from layout
//!   constants (break when a font, a padding or the composer's height
//!   changes); GPUI's `Animation` (fixed per element, cannot follow a
//!   route change).
//! - Cost: still at rest (the raster is cached by content); a layout
//!   change rasterises once; the journey is about eighteen frames.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{App, Bounds, Hsla, IntoElement, Pixels, Window};
use vessel_app_kit::vector::{self, Drawing};
use vessel_app_view::brand::layout::{NewSession, Session};
use vessel_app_view::brand::motion;
use vessel_app_view::brand::{self as view, Area, Figure, Pt, Star};

mod avatar;
#[expect(
    unused_imports,
    reason = "worker C mounts the avatar; the re-export is used with that line"
)]
pub(crate) use avatar::avatar;

/// The drawing's ink on the dark theme: the foreground at 7 % (the
/// study's quiet ink, recommended over 14 %).
const QUIET_DARK: f32 = 0.07;
/// On the light theme: the brand colour at 10 %.
const QUIET_LIGHT: f32 = 0.10;
/// The avatar's size, in points.
pub(crate) const AVATAR: f32 = 24.0;
/// The air between the frame and the column (worker C puts the avatar
/// inside the column, at the head of the reply).
const COLUMN_AIR: f32 = vessel_app_view::brand::ratios::CLEARANCE;

/// What the shown screen measured this frame.
#[derive(Debug, Clone, Copy)]
enum Seen {
    /// The heading, the composer's card and the pickers.
    New {
        heading: Area,
        composer: Area,
        pickers: Area,
    },
    /// The conversation's area and its column.
    Session(Session),
}

/// The first message's journey while it runs.
#[derive(Debug, Clone, Copy)]
struct Started {
    at: Instant,
    from: Figure,
}

#[derive(Debug, Default)]
struct Shared {
    seen: Cell<Option<Seen>>,
    /// The open session's visible conversation, this frame.
    viewport: Cell<Option<Area>>,
    /// The open session's column (the composer's), this frame.
    column: Cell<Option<Area>>,
    /// Where the coming reply's avatar will be (worker C's waiting row),
    /// this frame.
    reply_head: Cell<Option<Area>>,
    last_new: Cell<Option<Figure>>,
    journey: Cell<Option<Started>>,
}

/// The brand's state on the shell.
#[derive(Debug, Default, Clone)]
pub(crate) struct Brand {
    shared: Rc<Shared>,
}

fn area(bounds: Bounds<Pixels>) -> Area {
    Area {
        x: f32::from(bounds.origin.x),
        y: f32::from(bounds.origin.y),
        w: f32::from(bounds.size.width),
        h: f32::from(bounds.size.height),
    }
}

impl Brand {
    /// The first message was sent from the new session: the drawing
    /// starts its journey from the figure the new session last showed.
    pub(crate) fn sent(&self) {
        if let Some(from) = self.shared.last_new.get() {
            self.shared.journey.set(Some(Started {
                at: Instant::now(),
                from,
            }));
        }
    }

    /// For the new session's column (`on_children_prepainted`): after
    /// `skip` leading children come the heading, the composer's card and
    /// the pickers; everything after them (a note such as "no machine
    /// yet. Add one first.") counts with the pickers, so the V's bottom
    /// never passes under it (it did at 800 × 600 on the first run).
    pub(crate) fn new_session(
        &self,
        skip: usize,
    ) -> impl Fn(Vec<Bounds<Pixels>>, &mut Window, &mut App) + 'static {
        let shared = Rc::clone(&self.shared);
        move |children, _window, _cx| {
            let mut boxes = children.into_iter().skip(skip).map(area);
            if let (Some(heading), Some(composer), Some(pickers)) =
                (boxes.next(), boxes.next(), boxes.next())
            {
                shared.seen.set(Some(Seen::New {
                    heading,
                    composer,
                    pickers: boxes.fold(pickers, Area::union),
                }));
            }
        }
    }

    /// For the open session's screen (`on_children_prepainted`): child
    /// `viewport` is the conversation's visible area.
    #[expect(
        dead_code,
        reason = "the session's screen gives it to its body; this expectation goes with that line (worker C)"
    )]
    pub(crate) fn session_body(
        &self,
        viewport: usize,
    ) -> impl Fn(Vec<Bounds<Pixels>>, &mut Window, &mut App) + 'static {
        let shared = Rc::clone(&self.shared);
        move |children, _window, _cx| {
            shared
                .viewport
                .set(children.get(viewport).copied().map(area));
        }
    }

    /// For the wrapper of the conversation's column that holds the
    /// composer (`session::column`, `on_children_prepainted`): its one
    /// child is the column at its real width (820 points or the whole
    /// width, as the person's preference has it).
    #[expect(
        dead_code,
        reason = "the session's screen gives it to the composer's column; this expectation goes with that line (worker C)"
    )]
    pub(crate) fn session_column(
        &self,
    ) -> impl Fn(Vec<Bounds<Pixels>>, &mut Window, &mut App) + 'static {
        let shared = Rc::clone(&self.shared);
        move |children, _window, _cx| {
            shared.column.set(children.first().copied().map(area));
        }
    }

    /// For the row that waits for the agent's reply (worker C's "working"
    /// row, `on_children_prepainted`): its first child is the avatar's
    /// slot, where the first message's star lands and turns into the
    /// avatar; the reply's own avatar is drawn at the same place.
    #[expect(
        dead_code,
        reason = "worker C gives it to the waiting row; this expectation goes with that line"
    )]
    pub(crate) fn reply_head(
        &self,
    ) -> impl Fn(Vec<Bounds<Pixels>>, &mut Window, &mut App) + 'static {
        let shared = Rc::clone(&self.shared);
        move |children, _window, _cx| {
            shared.reply_head.set(children.first().copied().map(area));
        }
    }

    /// The drawing, to place first in the content panel (behind the
    /// screen; the panel clips it).
    pub(crate) fn backdrop(&self, tokens: &Tokens) -> impl IntoElement {
        let shared = Rc::clone(&self.shared);
        let ink = ink(tokens);
        vector::drawing(move |bounds, window, cx| {
            let figure = figure(&shared, area(bounds), vessel_app_kit::motion::held(cx))?;
            if shared.journey.get().is_some() {
                // One step per presented frame at the display's own rate
                // while the journey runs (the owner's word 42: a fixed
                // 30-a-second clock looked choppy); under held motion the
                // figure is already the end and nothing asks.
                window.request_animation_frame();
            }
            Some(Drawing {
                shapes: kit_shapes(&view::shapes(&figure)),
                ink,
            })
        })
    }
}

/// The figure this frame, from what the shown screen measured.
fn figure(shared: &Shared, panel: Area, still: bool) -> Option<Figure> {
    let (viewport, column) = (shared.viewport.take(), shared.column.take());
    let reply_head = shared.reply_head.take();
    let seen = shared.seen.take().or_else(|| {
        let (content, column) = (viewport?, column?);
        Some(Seen::Session(Session {
            content,
            column_left: column.x,
            column_right: column.right(),
            air: COLUMN_AIR,
        }))
    })?;
    match seen {
        Seen::New {
            heading,
            composer,
            pickers,
        } => {
            let figure = vessel_app_view::brand::layout::new_session(&NewSession {
                content: panel,
                composer,
                heading,
                pickers,
            });
            shared.last_new.set(Some(figure));
            shared.journey.set(None);
            Some(figure)
        }
        Seen::Session(screen) => {
            let to = vessel_app_view::brand::layout::session(&screen);
            let Some(started) = shared.journey.get() else {
                return Some(to);
            };
            let elapsed = slow(started.at.elapsed().as_secs_f32());
            if still || motion::ended(elapsed) {
                shared.journey.set(None);
                return Some(to);
            }
            let journey = motion::journey(elapsed, still);
            Some(motion::between(
                &started.from,
                &to,
                avatar_place(&screen, reply_head),
                journey,
            ))
        }
    }
}

/// Where the coming reply's avatar sits: the waiting row's slot when it
/// reported one this frame; else the column's left edge at the
/// conversation's first rows (the study's place).
fn avatar_place(screen: &Session, reported: Option<Area>) -> Star {
    if let Some(slot) = reported {
        #[expect(clippy::float_arithmetic, reason = "a slot's centre")]
        let centre = Pt {
            x: slot.x + slot.w / 2.0,
            y: slot.y + slot.h / 2.0,
        };
        return Star {
            centre,
            span: slot.w.min(slot.h),
        };
    }
    #[expect(clippy::float_arithmetic, reason = "a place at the column's left")]
    let centre = Pt {
        x: screen.column_left + AVATAR / 2.0,
        y: screen.content.y + 76.0,
    };
    Star {
        centre,
        span: AVATAR,
    }
}

/// The drawing's ink for the theme.
fn ink(tokens: &Tokens) -> Hsla {
    match tokens.scheme {
        vessel_app_kit::Scheme::Dark => tokens.neutral.foreground.opacity(QUIET_DARK),
        vessel_app_kit::Scheme::Light => {
            Hsla::from(vessel_app_kit::ui::rgb(vessel_app_kit::assets::BRAND)).opacity(QUIET_LIGHT)
        }
    }
}

fn kit_shapes(shapes: &[view::Shape]) -> Vec<vector::Shape> {
    shapes
        .iter()
        .map(|shape| vector::Shape {
            points: shape.points.iter().map(|p| (p.x, p.y)).collect(),
            closed: shape.closed,
            stroke: shape.stroke,
            alpha: shape.alpha,
        })
        .collect()
}

/// `VESSEL_APP_SLOW_MOTION=<n>` runs the journey n times slower, so a
/// capture on a virtual display can take its frames (as
/// `VESSEL_APP_REDUCE_MOTION` fixes the held state for a capture). Not a
/// setting: nothing is stored; unset or below 1, the journey's own speed.
#[expect(clippy::float_arithmetic, reason = "a time divided by a factor")]
pub(crate) fn slow(elapsed: f32) -> f32 {
    static FACTOR: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    let factor = *FACTOR.get_or_init(|| {
        std::env::var("VESSEL_APP_SLOW_MOTION")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(1.0)
            .max(1.0)
    });
    elapsed / factor
}

#[cfg(test)]
mod tests {
    use vessel_app_kit::motion::{QUICK, STANDARD};
    use vessel_app_view::brand::motion::{MARK, MORPH, MOVE};

    /// The journey's phases (`view`, which cannot read the kit) restate the
    /// kit's motion tokens; this keeps them one set.
    #[test]
    fn the_journey_reads_the_kits_motion_tokens() {
        assert!((MOVE - STANDARD.as_secs_f32()).abs() < 1e-6);
        assert!((MORPH - QUICK.as_secs_f32()).abs() < 1e-6);
        assert!((MARK - QUICK.as_secs_f32()).abs() < 1e-6);
    }
}
