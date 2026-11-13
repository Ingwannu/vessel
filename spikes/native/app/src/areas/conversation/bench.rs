//! A measurement of the timeline's scrolling, on when `VESSEL_APP_SCROLL_BENCH`
//! is set (the brief: "20,000 entries must scroll smoothly: measure it").
//!
//! [Decision Log]
//! - Purpose: a number for how long a frame takes while a long
//!   conversation scrolls, the same way on every machine.
//! - Alternatives: scrolling with synthetic input (`xdotool`), whose
//!   pacing is the tool's and not the app's; a profiler (says where time
//!   goes, not how long a frame is).
//! - Choice: once the session shown holds at least the number of entries
//!   the variable names, the timeline scrolls itself up by 40 points every
//!   frame from the end, asking for the next frame each time, and keeps
//!   the time between frames; after 600 frames it writes the median, the
//!   95th percentile and the longest to the log, once.
//! - Cost: off unless the variable is set; one `Instant` and one entry in
//!   a vector a frame while on.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use vessel_app_kit::VirtualList;
use vessel_app_kit::ui::{Window, px};

/// Frames measured.
const FRAMES: usize = 600;

/// Points scrolled a frame (upwards, from the end).
const STEP: f32 = -40.0;

/// The measurement's state.
#[derive(Debug, Default)]
pub(super) struct Bench {
    state: RefCell<State>,
}

#[derive(Debug, Default)]
struct State {
    last: Option<Instant>,
    frames: Vec<Duration>,
    done: bool,
}

/// The entries a session must hold before the measurement starts, when
/// it is on.
fn wanted() -> Option<usize> {
    std::env::var("VESSEL_APP_SCROLL_BENCH")
        .ok()
        .and_then(|value| value.parse().ok())
}

impl Bench {
    /// A frame of the timeline was drawn with `entries` entries: scroll on
    /// and ask for the next frame, while measuring.
    pub(super) fn frame(&self, entries: usize, list: &VirtualList, window: &Window) {
        let Some(wanted) = wanted() else {
            return;
        };
        let mut state = self.state.borrow_mut();
        if state.done || entries < wanted {
            return;
        }
        let now = Instant::now();
        if let Some(last) = state.last {
            state.frames.push(now.saturating_duration_since(last));
        }
        state.last = Some(now);
        if state.frames.len() >= FRAMES {
            state.done = true;
            let mut frames = std::mem::take(&mut state.frames);
            frames.sort_unstable();
            let at = |share: usize| {
                frames
                    .get(
                        frames
                            .len()
                            .saturating_mul(share)
                            .checked_div(100)
                            .unwrap_or(0),
                    )
                    .copied()
                    .unwrap_or_default()
            };
            tracing::info!(
                entries,
                rows = list.rows(),
                median = ?at(50),
                p95 = ?at(95),
                longest = ?frames.last().copied().unwrap_or_default(),
                "timeline scroll bench"
            );
            return;
        }
        list.scroll_by(px(STEP));
        window.request_animation_frame();
    }
}
