//! Mounted-row presentation only: the canonical text, identity and copy stay with
//! Timeline. No task/timer lives in a hidden session or crosses a resident epoch.
use std::time::{Duration, Instant};
use vessel_app_kit::ui::{App, ElementId, Window};
use vessel_app_view::timeline::{Entry, RevealClock, TextPhase, TextReveal};
use vessel_proto::EventSeq;

struct Kept {
    reveal: TextReveal,
    clock: RevealClock,
    version: u32,
    epoch: Instant,
}
impl Kept {
    fn begin(text: &str, version: u32) -> Self {
        Self {
            reveal: TextReveal::begin(text),
            clock: RevealClock::default(),
            version,
            epoch: Instant::now(),
        }
    }
    fn follow(
        &mut self,
        text: &str,
        version: u32,
        phase: TextPhase,
        now: Duration,
    ) -> (usize, bool) {
        let changed = self.version != version;
        if changed {
            self.reveal.retarget(text);
            self.version = version;
        }
        if phase == TextPhase::Complete {
            self.reveal.complete();
        } else if !changed
            && !self.reveal.settled()
            && let Some(elapsed) = self.clock.frame(now)
        {
            self.reveal.advance(elapsed);
        }
        let pending = !self.reveal.settled();
        if !pending {
            self.clock.reset();
        }
        (self.reveal.visible_end(text, phase), pending)
    }
}

/// A closing/status record clears Timeline::turn; older turns never reanimate.
pub(super) fn phase(turn: Option<EventSeq>, entry: &Entry) -> TextPhase {
    if turn.is_some_and(|turn| entry.seq >= turn) {
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
    let phase = if vessel_app_kit::motion::held(cx) {
        TextPhase::Complete
    } else {
        phase
    };
    let state = window.use_keyed_state(id, cx, |_, _| Kept::begin(text, version));
    let (end, pending) = state.update(cx, |kept, _| {
        kept.follow(text, version, phase, kept.epoch.elapsed())
    });
    if pending {
        window.request_animation_frame();
    }
    end
}

#[cfg(test)]
mod tests;
