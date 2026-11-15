//! Original waiting audio; no reference recording or binary asset is included.
//!
//! The controller creates this bounded immutable buffer once. Every repeat
//! becomes an ordinary Packet, sharing speech cancellation, reference and drain.

use super::{Refusal, packet::Packet};
use crate::audio::format::RateHz;
use bytes::Bytes;
use cpal::Sample as _;
use std::{fmt, time::Duration};
use vessel_app_view::voice::THINKING_CUE_DURATION;

const RATE_HZ: u32 = 24_000;
const MILLIS_PER_SECOND: u32 = 1_000;
const NANOS_PER_SECOND: u64 = 1_000_000_000;
const NOTE_TIME: Duration = Duration::from_millis(400);
const AMPLITUDE: f64 = 0.035;

struct Note {
    start: Duration,
    hz: u16,
}
const NOTES: [Note; 3] = [
    Note {
        start: Duration::ZERO,
        hz: 440,
    },
    Note {
        start: Duration::from_millis(620),
        hz: 554,
    },
    Note {
        start: Duration::from_millis(1240),
        hz: 659,
    },
];

#[derive(Debug, thiserror::Error)]
pub(in crate::audio) enum Failure {
    #[error("the waiting cue format cannot be represented")]
    Format,
    #[error("the waiting cue buffer could not be reserved")]
    Allocation(#[source] std::collections::TryReserveError),
}
pub(in crate::audio) struct Cue {
    pcm: Bytes,
    rate: RateHz,
}
impl fmt::Debug for Cue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WaitingCue")
            .field("bytes", &self.pcm.len())
            .finish_non_exhaustive()
    }
}
impl Cue {
    pub(in crate::audio) fn new() -> Result<Self, Failure> {
        let rate = RateHz::new(RATE_HZ).map_err(|_private| Failure::Format)?;
        let nanos =
            u64::try_from(THINKING_CUE_DURATION.as_nanos()).map_err(|_private| Failure::Format)?;
        let frames = nanos
            .checked_mul(u64::from(RATE_HZ))
            .and_then(|value| value.checked_div(NANOS_PER_SECOND))
            .ok_or(Failure::Format)?;
        let bytes = usize::try_from(frames)
            .ok()
            .and_then(|value| value.checked_mul(size_of::<i16>()))
            .ok_or(Failure::Format)?;
        let frames = u32::try_from(frames).map_err(|_private| Failure::Format)?;
        let mut pcm = Vec::new();
        pcm.try_reserve_exact(bytes).map_err(Failure::Allocation)?;
        for frame in 0..frames {
            let sample = sample(frame)?
                .clamp(-1.0, 1.0_f64.next_down())
                .to_sample::<i16>();
            pcm.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(Self {
            pcm: pcm.into(),
            rate,
        })
    }
    pub(in crate::audio) fn packet(&self) -> Result<Packet, Refusal> {
        Packet::new(self.pcm.clone(), self.rate)
    }
}
fn sample(frame: u32) -> Result<f64, Failure> {
    let duration = frames_for(NOTE_TIME)?;
    for note in &NOTES {
        let Some(local) = frame.checked_sub(frames_for(note.start)?) else {
            continue;
        };
        if local >= duration {
            continue;
        }
        let position = f64::from(local).mul_add(f64::from(duration).recip(), 0.0);
        // A squared-sine envelope begins/ends at silence, avoiding abrupt clicks.
        // Notes do not overlap; their maximum digital amplitude is explicit.
        let envelope = std::f64::consts::PI.mul_add(position, 0.0).sin().powi(2);
        let phase = std::f64::consts::TAU
            .mul_add(f64::from(note.hz), 0.0)
            .mul_add(f64::from(local), 0.0)
            .mul_add(f64::from(RATE_HZ).recip(), 0.0);
        return Ok(phase.sin().mul_add(envelope, 0.0).mul_add(AMPLITUDE, 0.0));
    }
    Ok(0.0)
}
fn frames_for(duration: Duration) -> Result<u32, Failure> {
    u32::try_from(duration.as_millis())
        .ok()
        .and_then(|ms| ms.checked_mul(RATE_HZ))
        .and_then(|samples| samples.checked_div(MILLIS_PER_SECOND))
        .ok_or(Failure::Format)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_pcm_has_the_shared_duration_smooth_silence_and_bounded_peak() {
        let cue = Cue::new().unwrap();
        assert_eq!(cue.packet().unwrap().duration(), THINKING_CUE_DURATION);
        assert_eq!(cue.pcm.len(), 143_040);
        let samples: Vec<_> = cue
            .pcm
            .as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| i16::from_le_bytes(*bytes))
            .collect();
        assert!(samples.iter().any(|sample| *sample != 0));
        assert!(samples.iter().all(|sample| sample.unsigned_abs() <= 1_147));
        assert_eq!(samples[0], 0);
        let silent_tail =
            usize::try_from(frames_for(Duration::from_millis(1640)).unwrap()).unwrap();
        assert!(samples[silent_tail..].iter().all(|sample| *sample == 0));
        assert!(!format!("{cue:?}").contains("[0, 0]"));
    }
}
