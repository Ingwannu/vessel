//! One-shot waiting cues: no idle timer and no late completion can restart a cue.
//!
//! The link owns actual playback and cancels only the cue's own receipt. This
//! policy reads no clock; supplied times share the current generation's origin.

use super::{Generation, Mute, Overlay, Phase, SpeechActivity, State};
use std::time::Duration;

const MIN_SILENCE: Duration = Duration::from_millis(1_500);
const REPEAT_GAP: Duration = Duration::from_millis(350);
/// Duration reserved for Vessel's independently generated cue, matching timing.
pub const THINKING_CUE_DURATION: Duration = Duration::from_millis(2_980);

/// Identity of a local cue, distinct from speech packets and remote receipts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CuePlayback {
    generation: Generation,
    ordinal: u64,
}

/// The device owner applies this only to the cue's timer or playback handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueAction {
    /// There is no new action; no background animation/timer is requested.
    None,
    /// Cancel the cue timer and its own pending/playing audio immediately.
    Cancel,
    /// Arm one timer at this generation-relative monotonic position.
    WaitUntil(Duration),
    /// Start one cue through the shared duplex output and AEC reference.
    Play(CuePlayback),
}

/// Invalid supplied time or identity exhaustion requires local cue cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CueError {
    /// The generation-relative clock went backwards or overflowed.
    #[error("voice cue clock is invalid")]
    Clock,
    /// Local cue identities cannot wrap and accept a stale receipt.
    #[error("voice cue identities exhausted")]
    Exhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Off,
    Waiting(Duration),
    Playing(CuePlayback),
}

/// One scheduler per live voice generation; no node or device is opened here.
#[derive(Debug)]
pub struct Cue {
    generation: Generation,
    ordinal: u64,
    mode: Mode,
    last: Duration,
}
impl Cue {
    /// Construct only for the generation returned by the voice start state.
    #[must_use]
    pub const fn new(generation: Generation) -> Self {
        Self {
            generation,
            ordinal: 0,
            mode: Mode::Off,
            last: Duration::ZERO,
        }
    }
    /// Reconcile on phase/activity changes and on the one armed timer.
    /// Mute does not suppress a listening user's wait cue, matching Paseo.
    ///
    /// # Errors
    /// Invalid time or an exhausted cue id; callers then cancel local cue audio.
    pub fn sync(
        &mut self,
        state: &State,
        speech: SpeechActivity,
        at: Duration,
    ) -> Result<CueAction, CueError> {
        self.clock(at)?;
        if !self.allowed(state, speech) {
            let action = if self.mode == Mode::Off {
                CueAction::None
            } else {
                CueAction::Cancel
            };
            self.mode = Mode::Off;
            return Ok(action);
        }
        match self.mode {
            Mode::Off => {
                let deadline = at.checked_add(MIN_SILENCE).ok_or(CueError::Clock)?;
                self.mode = Mode::Waiting(deadline);
                Ok(CueAction::WaitUntil(deadline))
            }
            Mode::Waiting(deadline) if at >= deadline => {
                self.ordinal = self.ordinal.checked_add(1).ok_or(CueError::Exhausted)?;
                let playback = CuePlayback {
                    generation: self.generation,
                    ordinal: self.ordinal,
                };
                self.mode = Mode::Playing(playback);
                Ok(CueAction::Play(playback))
            }
            Mode::Waiting(deadline) => Ok(CueAction::WaitUntil(deadline)),
            Mode::Playing(_) => Ok(CueAction::None),
        }
    }
    /// Schedule the next cue only after this cue's playback completed/failed.
    /// The reference waits cue duration +350ms AFTER its play promise settles.
    ///
    /// # Errors
    /// Invalid time; callers cancel local cue work rather than invent a deadline.
    pub fn finished(
        &mut self,
        state: &State,
        speech: SpeechActivity,
        playback: CuePlayback,
        at: Duration,
    ) -> Result<CueAction, CueError> {
        if self.mode != Mode::Playing(playback) {
            return Ok(CueAction::None);
        }
        self.clock(at)?;
        if !self.allowed(state, speech) {
            self.mode = Mode::Off;
            return Ok(CueAction::Cancel);
        }
        let deadline = at
            .checked_add(THINKING_CUE_DURATION)
            .and_then(|position| position.checked_add(REPEAT_GAP))
            .ok_or(CueError::Clock)?;
        self.mode = Mode::Waiting(deadline);
        Ok(CueAction::WaitUntil(deadline))
    }
    fn allowed(&self, state: &State, speech: SpeechActivity) -> bool {
        state.generation() == Some(self.generation)
            && state.phase() == Phase::Waiting
            && (speech == SpeechActivity::Quiet
                || matches!(state.overlay(), Overlay::Active(Mute::Muted)))
    }
    fn clock(&mut self, at: Duration) -> Result<(), CueError> {
        if at < self.last {
            return Err(CueError::Clock);
        }
        self.last = at;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
