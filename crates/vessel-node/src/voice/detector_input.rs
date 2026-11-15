//! Fixed16k PCM-to-inference window adaptation; no VAD or resampler is invented here.
//!
//! The shared PCM view owns byte alignment. This model edge owns exactly512
//! normalized samples per inference call, retaining one lookahead window across
//! client chunks. Detector implementation/distribution remains a separate gate.

use crate::speech_audio::{CodecError, Pcm16};
use std::fmt;
use tokio_util::sync::CancellationToken;
use vessel_proto::voice::CapturePcm;

pub(super) const WINDOW_SAMPLES: usize = 512;

pub(super) struct Window([f32; WINDOW_SAMPLES]);
impl Window {
    pub(super) fn samples(&self) -> &[f32; WINDOW_SAMPLES] {
        &self.0
    }
}
impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DetectorWindow([redacted])")
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum InputError {
    #[error("voice detector input was cancelled")]
    Cancelled,
    #[error("voice detector input alignment is invalid")]
    Codec(#[from] CodecError),
    #[error("voice detector window state is invalid")]
    Order,
}

pub(super) struct Windows {
    partial: Window,
    filled: usize,
}
impl Default for Windows {
    fn default() -> Self {
        Self {
            partial: Window([0.0; WINDOW_SAMPLES]),
            filled: 0,
        }
    }
}
impl fmt::Debug for Windows {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DetectorWindows")
            .field("partial_samples", &self.filled)
            .finish_non_exhaustive()
    }
}
impl Windows {
    pub(super) fn push(
        &mut self,
        chunk: &CapturePcm,
        stop: &CancellationToken,
        mut observe: impl FnMut(&Window),
    ) -> Result<(), InputError> {
        if stop.is_cancelled() {
            self.clear();
            return Err(InputError::Cancelled);
        }
        let pcm = Pcm16::new(chunk.as_slice())?;
        for bytes in pcm.as_slice().as_chunks::<2>().0 {
            if stop.is_cancelled() {
                self.clear();
                return Err(InputError::Cancelled);
            }
            // The pinned runtime consumes while buffered >512, not >=512:
            // retain a full final window until another sample actually arrives.
            if self.filled == WINDOW_SAMPLES {
                observe(&self.partial);
                self.clear();
                if stop.is_cancelled() {
                    return Err(InputError::Cancelled);
                }
            }
            let slot = self
                .partial
                .0
                .get_mut(self.filled)
                .ok_or(InputError::Order)?;
            *slot = f32::from(i16::from_le_bytes(*bytes)).mul_add(32_768.0_f32.recip(), 0.0);
            self.filled = self.filled.saturating_add(1);
        }
        if stop.is_cancelled() {
            self.clear();
            return Err(InputError::Cancelled);
        }
        Ok(())
    }
    /// Stop/reset discard retained lookahead, never pad or invent model input.
    pub(super) fn clear(&mut self) {
        self.partial.0.fill(0.0);
        self.filled = 0;
    }
}

#[cfg(test)]
mod tests;
