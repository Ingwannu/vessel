//! Display-only pacing from Paseo's agent-stream/text-reveal.ts. The canonical
//! message stays with Timeline; these counters never hold another transcript.
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation as _;

/// Backlog drains proportionally over Paseo's 150ms horizon.
const HORIZON: Duration = Duration::from_millis(150);
/// A stalled frame cannot accumulate an unbounded reveal step.
const MAX_ELAPSED: Duration = Duration::from_millis(250);
/// Integer nanoseconds approximate the source's 1000/60ms frame interval.
const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// Rendering phase, independent of text storage and provider completion claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPhase {
    /// More text can still arrive in the currently open turn.
    Streaming,
    /// History, finished work or an explicit full-content accessibility view.
    Complete,
}

/// Monotonic UTF-16 release units match JavaScript string indices. A visible
/// byte boundary is derived separately, so long graphemes cannot stall progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextReveal {
    target: usize,
    released: usize,
}
impl TextReveal {
    /// Hydrated history and a remounted row show everything on first sight.
    #[must_use]
    pub fn begin(text: &str) -> Self {
        let target = text.encode_utf16().count();
        Self {
            target,
            released: target,
        }
    }

    /// Only target growth is paced; a shorter replacement clamps the counter.
    pub fn retarget(&mut self, text: &str) {
        self.target = text.encode_utf16().count();
        self.released = self.released.min(self.target);
    }

    /// Release one frame proportionally, preserving a one-unit progress floor.
    pub fn advance(&mut self, elapsed: Duration) {
        self.released = self
            .released
            .saturating_add(step(self.target.saturating_sub(self.released), elapsed))
            .min(self.target);
    }

    /// Completion must not leave any received characters waiting for a clock.
    pub fn complete(&mut self) {
        self.released = self.target;
    }

    /// No frame is needed while the release counter has caught up.
    #[must_use]
    pub const fn settled(&self) -> bool {
        self.released >= self.target
    }

    /// End byte of the visible prefix in the same target supplied to retarget.
    /// Rust strings cannot contain the source's dangling high-surrogate case.
    #[must_use]
    pub fn visible_end(&self, text: &str, phase: TextPhase) -> usize {
        if phase == TextPhase::Complete {
            return text.len();
        }
        if self.settled() {
            return trailing_complete_end(text);
        }
        let mut units = 0_usize;
        for (byte, grapheme) in text.grapheme_indices(true) {
            units = units.saturating_add(grapheme.encode_utf16().count());
            if units > self.released {
                return byte;
            }
        }
        text.len()
    }
}

fn step(backlog: usize, elapsed: Duration) -> usize {
    let elapsed = elapsed.min(MAX_ELAPSED);
    if backlog == 0 || elapsed.is_zero() {
        return 0;
    }
    if elapsed >= HORIZON {
        return backlog;
    }
    let units = u128::try_from(backlog)
        .unwrap_or(u128::MAX)
        .saturating_mul(elapsed.as_nanos())
        .div_ceil(HORIZON.as_nanos());
    usize::try_from(units)
        .unwrap_or(backlog)
        .max(1)
        .min(backlog)
}

fn trailing_complete_end(text: &str) -> usize {
    let Some((byte, last)) = text.char_indices().next_back() else {
        return 0;
    };
    if last == '\u{200d}' {
        return byte;
    }
    let indicators = text
        .chars()
        .rev()
        .take_while(|character| ('\u{1f1e6}'..='\u{1f1ff}').contains(character))
        .count();
    if !indicators.is_multiple_of(2) {
        byte
    } else {
        text.len()
    }
}

/// Frame commits carry the timing remainder, limiting high-refresh screens to
/// the source's 60Hz pace. This holds no timer or application/clock capability.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RevealClock {
    last: Option<Duration>,
}
impl RevealClock {
    /// A newly growing settled row begins with one ordinary frame's elapsed.
    pub fn reset(&mut self) {
        self.last = None;
    }

    /// None means the hardware frame arrived before the next paced commit.
    #[must_use]
    pub fn frame(&mut self, now: Duration) -> Option<Duration> {
        let elapsed = self
            .last
            .map_or(FRAME_INTERVAL, |last| now.saturating_sub(last));
        if elapsed < FRAME_INTERVAL {
            return None;
        }
        let remainder = elapsed.as_nanos() % FRAME_INTERVAL.as_nanos();
        let remainder = Duration::from_nanos(u64::try_from(remainder).unwrap_or_default());
        self.last = Some(now.saturating_sub(remainder));
        Some(elapsed)
    }
}

#[cfg(test)]
mod tests;
