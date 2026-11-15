//! Interpret real Silero model probabilities before the outer turn boundary.
//!
//! Behavior reference: Sherpa1.12.28 silero-vad-model.cc88–160, used by pinned
//! Paseo. This is an independent closed-state expression, not an inference
//! engine or a peak detector. Equality/hysteresis edges are kept deliberately.

use super::{boundary::Detection, detector_input::WINDOW_SAMPLES};
use std::fmt;

const SPEECH_THRESHOLD: f32 = 0.5;
const CONTINUE_THRESHOLD: f32 = 0.35;
const MIN_SPEECH_SAMPLES: u64 = 1_600;
const MIN_SILENCE_SAMPLES: u64 = 3_200;

#[derive(Clone, Copy)]
pub(super) struct Probability(f32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("voice detector probability must be finite and normalized")]
pub(super) struct InvalidProbability;
impl Probability {
    pub(super) fn new(value: f32) -> Result<Self, InvalidProbability> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(InvalidProbability);
        }
        Ok(Self(value))
    }
}
impl fmt::Debug for Probability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SpeechProbability([redacted])")
    }
}
#[derive(Debug, Clone, Copy, Default)]
enum Phase {
    #[default]
    Quiet,
    Confirming(u64),
    Speaking {
        quiet_since: Option<u64>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("voice detector sample clock exhausted")]
pub(super) struct ClockExhausted;
#[derive(Debug, Default)]
pub(super) struct Gate {
    position: u64,
    phase: Phase,
}
impl Gate {
    pub(super) fn observe(
        &mut self,
        probability: Probability,
    ) -> Result<Detection, ClockExhausted> {
        let window = u64::try_from(WINDOW_SAMPLES).map_err(|_private| ClockExhausted)?;
        self.position = self.position.checked_add(window).ok_or(ClockExhausted)?;
        let score = probability.0;
        match self.phase {
            Phase::Quiet | Phase::Confirming(_) if score < SPEECH_THRESHOLD => {
                self.phase = Phase::Quiet;
                Ok(Detection::Quiet)
            }
            Phase::Quiet if score > SPEECH_THRESHOLD => {
                self.phase = Phase::Confirming(self.position);
                Ok(Detection::Quiet)
            }
            Phase::Confirming(since) if score > SPEECH_THRESHOLD => {
                if self.position.saturating_sub(since) >= MIN_SPEECH_SAMPLES {
                    self.phase = Phase::Speaking { quiet_since: None };
                    Ok(Detection::Speech)
                } else {
                    Ok(Detection::Quiet)
                }
            }
            Phase::Quiet | Phase::Confirming(_) => Ok(Detection::Quiet),
            Phase::Speaking { quiet_since } => {
                if score > SPEECH_THRESHOLD {
                    self.phase = Phase::Speaking { quiet_since: None };
                    return Ok(Detection::Speech);
                }
                if score > CONTINUE_THRESHOLD {
                    return Ok(Detection::Speech);
                }
                let since = quiet_since.unwrap_or(self.position);
                if self.position.saturating_sub(since) >= MIN_SILENCE_SAMPLES {
                    self.phase = Phase::Quiet;
                    Ok(Detection::Quiet)
                } else {
                    self.phase = Phase::Speaking {
                        quiet_since: Some(since),
                    };
                    Ok(Detection::Speech)
                }
            }
        }
    }
    /// The inference adapter must reset its recurrent state at the same boundary.
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }
}
#[cfg(test)]
mod tests;
