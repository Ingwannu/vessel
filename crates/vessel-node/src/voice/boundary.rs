//! Paseo's outer Silero boundary state machine, independent of inference.
//!
//! Silero's own minimum speech/silence processing is not reproduced here:
//! this consumes its detected state once per 512-sample window at 16 kHz.
//! Counting samples avoids a drifting wall clock during buffered uploads.

const WINDOW_SAMPLES: u64 = 512;
const CONFIRM_SAMPLES: u64 = 12_800;
const SILENCE_SAMPLES: u64 = 16_000;

/// Detector observations are policy inputs, never inferred from PCM peak here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Detection {
    Speech,
    Quiet,
}

/// The actor uses confirmed edges to control interruption and transcription.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BoundaryEvent {
    NoChange,
    SpeechStarted,
    SpeechStopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Position(u64);

impl Position {
    fn advance(self) -> Result<Self, ClockExhausted> {
        self.0
            .checked_add(WINDOW_SAMPLES)
            .map(Self)
            .ok_or(ClockExhausted)
    }

    fn elapsed(self, since: Self) -> u64 {
        // Both positions belong to this monotonically increasing detector clock.
        self.0.saturating_sub(since.0)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    Confirming {
        since: Position,
    },
    Speaking,
    Ending {
        since: Position,
    },
}

/// Overflow retires the lease; a clock must never wrap into a previous turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("voice detector sample clock exhausted")]
pub(super) struct ClockExhausted;

/// One actor owns this boundary clock and its inference instance.
#[derive(Debug)]
pub(super) struct Boundary {
    position: Position,
    phase: Phase,
}

impl Default for Boundary {
    fn default() -> Self {
        Self {
            position: Position(0),
            phase: Phase::Idle,
        }
    }
}

impl Boundary {
    /// Consume exactly one inference window; no time elapses merely because the node waited.
    pub(super) fn observe(&mut self, detected: Detection) -> Result<BoundaryEvent, ClockExhausted> {
        self.position = self.position.advance()?;
        let (phase, event) = match (self.phase, detected) {
            (Phase::Idle, Detection::Speech) => (
                Phase::Confirming {
                    since: self.position,
                },
                BoundaryEvent::NoChange,
            ),
            (Phase::Idle | Phase::Confirming { .. }, Detection::Quiet) => {
                (Phase::Idle, BoundaryEvent::NoChange)
            }
            (Phase::Confirming { since }, Detection::Speech) => {
                if self.position.elapsed(since) >= CONFIRM_SAMPLES {
                    (Phase::Speaking, BoundaryEvent::SpeechStarted)
                } else {
                    (Phase::Confirming { since }, BoundaryEvent::NoChange)
                }
            }
            (Phase::Speaking | Phase::Ending { .. }, Detection::Speech) => {
                (Phase::Speaking, BoundaryEvent::NoChange)
            }
            (Phase::Speaking, Detection::Quiet) => (
                Phase::Ending {
                    since: self.position,
                },
                BoundaryEvent::NoChange,
            ),
            (Phase::Ending { since }, Detection::Quiet) => {
                if self.position.elapsed(since) >= SILENCE_SAMPLES {
                    (Phase::Idle, BoundaryEvent::SpeechStopped)
                } else {
                    (Phase::Ending { since }, BoundaryEvent::NoChange)
                }
            }
        };
        self.phase = phase;
        Ok(event)
    }

    /// Flush seals only confirmed speech; a short false start never becomes a prompt.
    pub(super) fn flush(&mut self) -> BoundaryEvent {
        let event = match self.phase {
            Phase::Speaking | Phase::Ending { .. } => BoundaryEvent::SpeechStopped,
            Phase::Idle | Phase::Confirming { .. } => BoundaryEvent::NoChange,
        };
        self.phase = Phase::Idle;
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observe_many(
        boundary: &mut Boundary,
        detection: Detection,
        windows: usize,
    ) -> Vec<BoundaryEvent> {
        (0..windows)
            .map(|_| boundary.observe(detection).unwrap())
            .filter(|event| *event != BoundaryEvent::NoChange)
            .collect()
    }

    #[test]
    fn confirmation_and_ending_use_the_reference_window_clock() {
        let mut boundary = Boundary::default();
        // First detection begins confirmation; 25 more windows reach exactly 800 ms.
        assert!(observe_many(&mut boundary, Detection::Speech, 25).is_empty());
        assert_eq!(
            boundary.observe(Detection::Speech).unwrap(),
            BoundaryEvent::SpeechStarted
        );
        // First silence begins ending; 32 further windows are the first past 1000 ms.
        assert!(observe_many(&mut boundary, Detection::Quiet, 32).is_empty());
        assert_eq!(
            boundary.observe(Detection::Quiet).unwrap(),
            BoundaryEvent::SpeechStopped
        );
        assert_eq!(boundary.flush(), BoundaryEvent::NoChange);
    }

    #[test]
    fn false_start_is_discarded_and_resumed_speech_cancels_ending() {
        let mut boundary = Boundary::default();
        assert!(observe_many(&mut boundary, Detection::Speech, 20).is_empty());
        assert_eq!(boundary.flush(), BoundaryEvent::NoChange);
        assert_eq!(
            observe_many(&mut boundary, Detection::Speech, 26),
            vec![BoundaryEvent::SpeechStarted]
        );
        assert!(observe_many(&mut boundary, Detection::Quiet, 20).is_empty());
        assert!(observe_many(&mut boundary, Detection::Speech, 1).is_empty());
        assert!(observe_many(&mut boundary, Detection::Quiet, 32).is_empty());
        assert_eq!(
            boundary.observe(Detection::Quiet).unwrap(),
            BoundaryEvent::SpeechStopped
        );
    }

    #[test]
    fn flush_closes_confirmed_speech_once_and_overflow_never_wraps() {
        let mut boundary = Boundary::default();
        assert_eq!(
            observe_many(&mut boundary, Detection::Speech, 26),
            vec![BoundaryEvent::SpeechStarted]
        );
        assert_eq!(boundary.flush(), BoundaryEvent::SpeechStopped);
        assert_eq!(boundary.flush(), BoundaryEvent::NoChange);
        boundary.position = Position(u64::MAX);
        assert_eq!(boundary.observe(Detection::Quiet), Err(ClockExhausted));
        assert_eq!(boundary.position, Position(u64::MAX));
    }
}
