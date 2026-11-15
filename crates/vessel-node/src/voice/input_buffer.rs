//! Bounded idle pre-roll preserves confirmation audio without retaining endless silence.
//!
//! This owns turn buffering policy only. Shared `speech_audio` owns sample
//! interpretation/resampling/WAV; no copy of that codec is introduced here.

use std::collections::VecDeque;
use std::fmt;

use vessel_proto::voice::CapturePcm;

const PRE_ROLL_BYTES: usize = 64_000;
pub(super) const MAX_UTTERANCE_BYTES: usize = 1_920_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum BufferError {
    #[error("spoken utterance exceeds sixty seconds including pre-roll")]
    TooLong,
    #[error("voice audio memory could not be reserved")]
    Allocation,
    #[error("voice speech boundary arrived out of order")]
    Order,
}

pub(super) struct Utterance(Vec<u8>);
impl Utterance {
    pub(super) fn pcm16(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Debug for Utterance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Utterance")
            .field("bytes", &self.0.len())
            .finish()
    }
}

#[derive(Default)]
enum Phase {
    #[default]
    Listening,
    Capturing(Utterance),
}

#[derive(Default)]
pub(super) struct InputBuffer {
    pre_roll: VecDeque<u8>,
    phase: Phase,
}
impl fmt::Debug for InputBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputBuffer")
            .field("pre_roll_bytes", &self.pre_roll.len())
            .field("capturing", &matches!(self.phase, Phase::Capturing(_)))
            .finish()
    }
}

impl InputBuffer {
    pub(super) fn append(&mut self, pcm: &CapturePcm) -> Result<(), BufferError> {
        match &mut self.phase {
            Phase::Listening => {
                // Reserve before retiring old bytes: allocation failure must
                // leave the current prefix and turn state untouched.
                let length = self
                    .pre_roll
                    .len()
                    .checked_add(pcm.as_slice().len())
                    .ok_or(BufferError::TooLong)?;
                self.pre_roll
                    .try_reserve(pcm.as_slice().len())
                    .map_err(|_private| BufferError::Allocation)?;
                self.pre_roll.extend(pcm.as_slice());
                let excess = length.saturating_sub(PRE_ROLL_BYTES);
                let _retired = self.pre_roll.drain(..excess);
            }
            Phase::Capturing(audio) => {
                let length = audio
                    .0
                    .len()
                    .checked_add(pcm.as_slice().len())
                    .ok_or(BufferError::TooLong)?;
                if length > MAX_UTTERANCE_BYTES {
                    return Err(BufferError::TooLong);
                }
                audio
                    .0
                    .try_reserve(pcm.as_slice().len())
                    .map_err(|_private| BufferError::Allocation)?;
                audio.0.extend_from_slice(pcm.as_slice());
            }
        }
        Ok(())
    }

    /// Called after appending the chunk whose VAD windows confirmed speech.
    pub(super) fn confirmed(&mut self) -> Result<(), BufferError> {
        if matches!(self.phase, Phase::Capturing(_)) {
            return Err(BufferError::Order);
        }
        let mut audio = Vec::new();
        audio
            .try_reserve(self.pre_roll.len())
            .map_err(|_private| BufferError::Allocation)?;
        audio.extend(self.pre_roll.drain(..));
        self.phase = Phase::Capturing(Utterance(audio));
        Ok(())
    }

    /// Only the ending-silence edge seals confirmed speech; idle silence has no STT job.
    pub(super) fn seal(&mut self) -> Option<Utterance> {
        match std::mem::take(&mut self.phase) {
            Phase::Listening => None,
            Phase::Capturing(audio) => Some(audio),
        }
    }

    /// Stop/abort discards rather than committing unsent microphone audio.
    pub(super) fn discard(&mut self) {
        self.pre_roll.clear();
        self.phase = Phase::Listening;
    }
}

#[cfg(test)]
mod tests;
