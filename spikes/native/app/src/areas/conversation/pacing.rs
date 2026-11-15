//! Mounted-row presentation only: the canonical text, identity and copy stay with
//! Timeline. No task/timer lives in a hidden session or crosses a resident epoch.
use super::ConversationArea;
use std::time::{Duration, Instant};
use vessel_app_kit::ui::{App, ElementId, Window};
use vessel_app_view::timeline::{Entry, RevealClock, TextPhase, TextReveal};

#[derive(Clone, Copy, PartialEq, Eq)]
enum FrameWake {
    Idle,
    Scheduled,
    Ready,
}

struct Kept {
    reveal: TextReveal,
    clock: RevealClock,
    version: u32,
    epoch: Instant,
    wake: FrameWake,
}
impl Kept {
    fn begin(text: &str, version: u32) -> Self {
        Self {
            reveal: TextReveal::begin(text),
            clock: RevealClock::default(),
            version,
            epoch: Instant::now(),
            wake: FrameWake::Idle,
        }
    }
    fn follow(
        &mut self,
        text: &str,
        version: u32,
        phase: TextPhase,
        now: Duration,
    ) -> (usize, bool) {
        let frame_ready = match self.wake {
            FrameWake::Idle | FrameWake::Scheduled => false,
            FrameWake::Ready => {
                self.wake = FrameWake::Idle;
                true
            }
        };
        let changed = self.version != version;
        if changed {
            self.reveal.retarget(text);
            self.version = version;
        }
        match phase {
            TextPhase::Complete => self.reveal.complete(),
            TextPhase::Streaming => {
                if frame_ready
                    && !changed
                    && !self.reveal.settled()
                    && let Some(elapsed) = self.clock.frame(now)
                {
                    self.reveal.advance(elapsed);
                }
            }
        }
        let pending = !self.reveal.settled();
        if !pending {
            self.clock.reset();
        }
        (self.reveal.visible_end(text, phase), pending)
    }
}

/// Read-only snapshots are history. A closing/status record clears the managed
/// `Timeline::turn`; older turns never reanimate.
pub(super) fn phase(area: &ConversationArea, entry: &Entry) -> TextPhase {
    if area.shown.is_some() && area.timeline.turn().is_some_and(|turn| entry.seq >= turn) {
        TextPhase::Streaming
    } else {
        TextPhase::Complete
    }
}

/// A displayed prefix over the SAME target, cached only while this row is drawn.
/// First sight is full; target growth waits for a frame, and completion is instant.
pub(super) fn visible_end(
    id: ElementId,
    text: &str,
    version: u32,
    phase: TextPhase,
    window: &mut Window,
    cx: &mut App,
) -> usize {
    visible_end_with_clock(
        id,
        text,
        version,
        phase,
        |kept| kept.epoch.elapsed(),
        window,
        cx,
    )
}

// The same renderer accepts a deterministic frame clock in its GPUI fixture;
// production uses its mounted epoch. No clock or target leaves this row.
fn visible_end_with_clock(
    id: ElementId,
    text: &str,
    version: u32,
    phase: TextPhase,
    now: impl FnOnce(&Kept) -> Duration,
    window: &mut Window,
    cx: &mut App,
) -> usize {
    let phase = if vessel_app_kit::motion::held(cx) {
        TextPhase::Complete
    } else {
        phase
    };
    let state = window.use_keyed_state(id, cx, |_, _| Kept::begin(text, version));
    let (end, schedule) = state.update(cx, |kept, _| {
        let now = now(kept);
        let (end, pending) = kept.follow(text, version, phase, now);
        let schedule = pending && kept.wake != FrameWake::Scheduled;
        if schedule {
            kept.wake = FrameWake::Scheduled;
        }
        (end, schedule)
    });
    if schedule {
        let weak = state.downgrade();
        window.on_next_frame(move |_, cx| {
            if let Some(state) = weak.upgrade() {
                state.update(cx, |kept, cx| {
                    if kept.reveal.settled() {
                        kept.wake = FrameWake::Idle;
                    } else {
                        kept.wake = FrameWake::Ready;
                        cx.notify();
                    }
                });
            }
        });
    }
    end
}

#[cfg(test)]
mod tests;
