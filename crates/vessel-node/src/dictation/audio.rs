//! Dictation-only bounded segments and silence, using the shared speech codec.
//!
//! No files, logging or HTTP. The shared codec owns interpolation and RIFF;
//! this module owns the dictation commit threshold, peak and detached segments.

pub(super) use crate::speech_audio::ResampledPcm;
use crate::speech_audio::{CodecError, Pcm16, Wav};
use bytes::Bytes;
use std::{fmt, time::Duration};
use vessel_proto::dictation::{DictationPcm, SampleRate};

/// HTTP fixtures verify that the real WAV encoder supplies the provider rate.
pub(super) const PROVIDER_HZ: u32 = crate::speech_audio::PROVIDER_RATE.get();
const SAMPLE_BYTES: usize = size_of::<i16>();
pub(super) const COMMIT_SECONDS: usize = 15;
// Commit after appending a whole second-sized input chunk, as Paseo does.
const MAX_SEGMENT_SECONDS: usize = COMMIT_SECONDS + super::MAX_CHUNK_SECONDS;
const SILENCE_PEAK: u16 = 300;

/// Preserve numeric causes without retaining any audio in an error.
#[derive(Debug, thiserror::Error)]
pub(super) enum AudioError {
    /// Typed PCM guarantees complete samples; retain a decoder defect's cause.
    #[error("dictation PCM did not contain complete samples")]
    SampleWidth(#[source] std::array::TryFromSliceError),
    /// Shared numeric, alignment and RIFF causes remain typed without samples.
    #[error("the dictation speech codec refused the audio")]
    Codec(#[from] CodecError),
    /// Refuse before appending when a bounded segment cannot be allocated.
    #[error("the dictation segment could not be reserved")]
    Allocation(#[source] std::collections::TryReserveError),
    /// Keep count conversions checked on every supported target.
    #[error("dictation audio counts exceeded this platform's range")]
    Count(#[source] std::num::TryFromIntError),
    /// Bound provider work even if the stream's shape validation regresses.
    #[error("a dictation provider segment exceeds {max} seconds", max = MAX_SEGMENT_SECONDS)]
    SegmentTooLong,
}

/// Adapt the feature's already validated capture type to the shared borrowed view.
pub(super) struct Resampler(crate::speech_audio::Resampler);
impl Resampler {
    pub(super) fn new(input: SampleRate) -> Self {
        Self(crate::speech_audio::Resampler::new(input))
    }
    pub(super) fn push<'a>(
        &mut self,
        pcm: &'a DictationPcm,
    ) -> Result<ResampledPcm<'a>, AudioError> {
        self.0
            .push(Pcm16::new(pcm.as_slice())?)
            .map_err(AudioError::Codec)
    }
}

/// One commit's audio is detached before HTTP can await or another chunk arrives.
#[derive(Clone)]
pub(super) struct Segment(Bytes);

impl fmt::Debug for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Segment([redacted])")
    }
}

impl Segment {
    /// Form fixtures use parsed PCM without depending on silence or resampling.
    #[cfg(test)]
    pub(super) fn from_test_pcm(pcm: DictationPcm) -> Self {
        Self(pcm.into_bytes())
    }

    /// Encode WAV directly into the multipart body, avoiding a second WAV copy.
    pub(super) fn append_wav(&self, output: &mut Vec<u8>) -> Result<(), AudioError> {
        Wav::new(crate::speech_audio::PROVIDER_RATE, Pcm16::new(&self.0)?)?
            .append_to(output)
            .map_err(AudioError::Codec)
    }

    /// Multipart framing checks its generated delimiter against binary content.
    pub(super) fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

/// An actor's open segment is bounded even while an endpoint is backpressured.
pub(super) struct SegmentBuffer {
    pcm: Vec<u8>,
    peak: u16,
    bytes_per_second: usize,
}

impl SegmentBuffer {
    /// Convert the single provider-rate constant once, with no numeric cast.
    pub(super) fn new() -> Result<Self, AudioError> {
        let rate = usize::try_from(PROVIDER_HZ).map_err(AudioError::Count)?;
        Ok(Self {
            pcm: Vec::new(),
            peak: 0,
            bytes_per_second: rate.saturating_mul(SAMPLE_BYTES),
        })
    }

    /// Refuse before retaining a chunk that would exceed the segment bound.
    pub(super) fn append(&mut self, chunk: &ResampledPcm<'_>) -> Result<(), AudioError> {
        let chunk = chunk.as_pcm();
        if self.pcm.len().saturating_add(chunk.as_slice().len())
            > self.bytes_per_second.saturating_mul(MAX_SEGMENT_SECONDS)
        {
            return Err(AudioError::SegmentTooLong);
        }
        self.pcm
            .try_reserve(chunk.as_slice().len())
            .map_err(AudioError::Allocation)?;
        for pair in chunk.as_slice().chunks_exact(SAMPLE_BYTES) {
            let pair = <[u8; SAMPLE_BYTES]>::try_from(pair).map_err(AudioError::SampleWidth)?;
            self.peak = self.peak.max(i16::from_le_bytes(pair).unsigned_abs());
        }
        self.pcm.extend_from_slice(chunk.as_slice());
        Ok(())
    }

    /// Commit after the whole current chunk, preserving Paseo's threshold order.
    pub(super) fn ready(&self) -> bool {
        self.pcm.len() >= self.bytes_per_second.saturating_mul(COMMIT_SECONDS)
    }

    /// Supply finish-budget arithmetic with a duration rather than raw bytes.
    pub(super) fn duration(&self) -> Result<Duration, AudioError> {
        let seconds = self.pcm.len().div_ceil(self.bytes_per_second);
        Ok(Duration::from_secs(
            u64::try_from(seconds).map_err(AudioError::Count)?,
        ))
    }

    /// Silence is intentionally discarded at a commit boundary or finish.
    pub(super) fn seal(&mut self) -> Option<Segment> {
        let pcm = std::mem::take(&mut self.pcm);
        let peak = std::mem::replace(&mut self.peak, 0);
        if pcm.is_empty() || peak < SILENCE_PEAK {
            None
        } else {
            Some(Segment(pcm.into()))
        }
    }
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod tests;
