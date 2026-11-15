//! Bounded display telemetry; server VAD activity is separate from sound level.
//!
//! The reference smooths from the last published level, not every hidden
//! capture sample. Time and level are supplied values; this module reads no clock.

use std::time::Duration;

use super::Mute;

const PUBLISH_INTERVAL: Duration = Duration::from_millis(120);
const CHANGE_EPSILON: f32 = 0.02;
const ATTACK: f32 = 0.35;
const ATTACK_REMAINDER: f32 = 0.65;
const RELEASE: f32 = 0.18;
const RELEASE_REMAINDER: f32 = 0.82;
const DECIMAL_SCALE: f32 = 1_000.0;
const DECIMAL_RECIPROCAL: f32 = 0.001;

/// A finite normalized audio display level. It contains no retained PCM.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Level(f32);

/// Non-finite/out-of-range levels cannot reach drawing arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("audio display level must be finite and between zero and one")]
pub struct InvalidLevel;

impl Level {
    /// Validate measurements at the capture/telemetry boundary.
    ///
    /// # Errors
    /// NaN, infinity and values outside the normalized range are refused.
    pub fn new(raw: f32) -> Result<Self, InvalidLevel> {
        if !raw.is_finite() || !(0.0..=1.0).contains(&raw) {
            return Err(InvalidLevel);
        }
        Ok(Self(raw))
    }

    /// The normalized value, only for an explicitly validated kit drawing adapter.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Activity comes from the node's confirmed detector, never a local peak threshold.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SpeechActivity {
    /// The server has not confirmed speech.
    #[default]
    Quiet,
    /// The node confirmed an utterance in progress.
    Speaking,
}

/// All three display inputs are typed; mute does not discard the raw meter state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Telemetry {
    /// The smoothed published level.
    pub level: Level,
    /// Server-confirmed speech state.
    pub activity: SpeechActivity,
    /// Muted rendering is static even if physical capture reports sound.
    pub mute: Mute,
}

/// One meter per active generation; replacing the generation constructs a new meter.
#[derive(Debug, Default)]
pub struct DisplayMeter {
    published: Level,
    last_publish: Duration,
}

impl DisplayMeter {
    /// Follow Paseo's attack/release and 120 ms / 0.02 publication thresholds.
    /// Hidden updates leave the published state unchanged.
    pub fn observe(&mut self, input: Level, at: Duration) -> Option<Level> {
        let previous = self.published.get();
        let (weight, remainder) = if input.get() >= previous {
            (ATTACK, ATTACK_REMAINDER)
        } else {
            (RELEASE, RELEASE_REMAINDER)
        };
        let next = input
            .get()
            .mul_add(weight, previous.mul_add(remainder, 0.0))
            .clamp(0.0, 1.0);
        let changed = previous.mul_add(-1.0, next).abs() >= CHANGE_EPSILON;
        if !changed && at.saturating_sub(self.last_publish) < PUBLISH_INTERVAL {
            return None;
        }
        self.published = Level(
            next.mul_add(DECIMAL_SCALE, 0.0)
                .round()
                .mul_add(DECIMAL_RECIPROCAL, 0.0)
                .clamp(0.0, 1.0),
        );
        self.last_publish = at;
        Some(self.published)
    }

    /// Controls/VAD may redraw immediately without manufacturing a volume update.
    #[must_use]
    pub const fn telemetry(&self, mute: Mute, activity: SpeechActivity) -> Telemetry {
        Telemetry {
            level: self.published,
            activity,
            mute,
        }
    }
}

#[cfg(test)]
mod tests;
