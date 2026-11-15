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

/// Fallible inference stops the current chunk at its first failed window.
#[derive(Debug, thiserror::Error)]
pub(super) enum PushError<E> {
    #[error(transparent)]
    Input(#[from] InputError),
    #[error("voice detector refused a window")]
    Observer(#[source] E),
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
        match self.try_push(chunk, stop, |window| {
            observe(window);
            Ok::<(), std::convert::Infallible>(())
        }) {
            Ok(()) => Ok(()),
            Err(PushError::Input(error)) => Err(error),
            Err(PushError::Observer(never)) => match never {},
        }
    }
    pub(super) fn try_push<E>(
        &mut self,
        chunk: &CapturePcm,
        stop: &CancellationToken,
        mut observe: impl FnMut(&Window) -> Result<(), E>,
    ) -> Result<(), PushError<E>> {
        if stop.is_cancelled() {
            self.clear();
            return Err(InputError::Cancelled.into());
        }
        let pcm = Pcm16::new(chunk.as_slice()).map_err(InputError::from)?;
        for bytes in pcm.as_slice().as_chunks::<2>().0 {
            if stop.is_cancelled() {
                self.clear();
                return Err(InputError::Cancelled.into());
            }
            // The pinned runtime consumes while buffered >512, not >=512:
            // retain a full final window until another sample actually arrives.
            if self.filled == WINDOW_SAMPLES {
                let observed = observe(&self.partial);
                self.clear();
                observed.map_err(PushError::Observer)?;
                if stop.is_cancelled() {
                    return Err(InputError::Cancelled.into());
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
            return Err(InputError::Cancelled.into());
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
