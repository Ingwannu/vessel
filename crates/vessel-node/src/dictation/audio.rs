//! In-memory provider PCM: integer resampling, silence, and bounded segments.
//!
//! No files, logging or HTTP. Provider-rate audio has its own type; integer
//! phase avoids unchecked floating arithmetic. ADR 0067 records the limited
//! synthetic comparison and its rounding differences from Paseo.

use bytes::Bytes;
use std::{fmt, time::Duration};
use vessel_proto::dictation::{DictationPcm, SampleRate};

/// HTTP fixtures verify that the real WAV encoder supplies the provider rate.
pub(super) const PROVIDER_HZ: u32 = 24_000;
const SAMPLE_BYTES: usize = size_of::<i16>();
const WAV_HEADER_BYTES: usize = 44;
const RIFF_PREFIX_BYTES: usize = 8;
const WAVE_FMT_BYTES: u32 = 16;
const PCM_FORMAT: u16 = 1;
const MONO_CHANNELS: u16 = 1;
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
    /// Conversion failure must never become a wrapped or truncated sample.
    #[error("dictation resampling produced an invalid sample")]
    SampleValue(#[source] std::num::TryFromIntError),
    /// Keep count conversions checked on every supported target.
    #[error("dictation audio counts exceeded this platform's range")]
    Count(#[source] std::num::TryFromIntError),
    /// Bound provider work even if the stream's shape validation regresses.
    #[error("a dictation provider segment exceeds {max} seconds", max = MAX_SEGMENT_SECONDS)]
    SegmentTooLong,
    /// RIFF has a fixed width size field even on wider pointer targets.
    #[error("the dictation WAV exceeds its size field")]
    WaveTooLong,
}

/// Provider-rate PCM cannot be confused with a client's original-rate chunk.
#[derive(Clone)]
pub(super) struct ResampledPcm(Bytes);

impl fmt::Debug for ResampledPcm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ResampledPcm([redacted])")
    }
}

/// Carry interpolation across calls, without retaining the old chunk's buffer.
pub(super) struct Resampler {
    input: SampleRate,
    phase: u32,
    left: Option<i16>,
}

impl Resampler {
    /// Only a parsed wire rate can select the interpolation ratio.
    pub(super) fn new(input: SampleRate) -> Self {
        Self {
            input,
            phase: 0,
            left: None,
        }
    }

    /// Preserve one-sample tails and phase across independently decoded chunks.
    pub(super) fn push(&mut self, pcm: &DictationPcm) -> Result<ResampledPcm, AudioError> {
        if self.input.get() == PROVIDER_HZ {
            return Ok(ResampledPcm(pcm.clone().into_bytes()));
        }
        // Reserve from this parsed rate, rather than overallocating every
        // downsampled chunk for the most expansive supported rate.
        let mut output = Vec::with_capacity(output_capacity(pcm, self.input)?);
        for sample in pcm.as_slice().chunks_exact(SAMPLE_BYTES) {
            let pair = <[u8; SAMPLE_BYTES]>::try_from(sample).map_err(AudioError::SampleWidth)?;
            let right = i16::from_le_bytes(pair);
            if let Some(left) = self.left {
                while self.phase < PROVIDER_HZ {
                    let sample = interpolate(left, right, self.phase)?;
                    output.extend_from_slice(&sample.to_le_bytes());
                    // The cursor is bounded by the provider and input rate caps.
                    // Saturation also prevents a future invalid rate from wrapping.
                    self.phase = self.phase.saturating_add(self.input.get());
                }
                self.phase = self.phase.saturating_sub(PROVIDER_HZ);
            }
            self.left = Some(right);
        }
        Ok(ResampledPcm(output.into()))
    }
}

fn output_capacity(pcm: &DictationPcm, rate: SampleRate) -> Result<usize, AudioError> {
    let samples =
        u64::try_from(pcm.as_slice().len().div_euclid(SAMPLE_BYTES)).map_err(AudioError::Count)?;
    let width = u64::try_from(SAMPLE_BYTES).map_err(AudioError::Count)?;
    let bytes = samples
        .saturating_mul(u64::from(PROVIDER_HZ))
        .div_ceil(u64::from(rate.get()))
        .saturating_mul(width);
    usize::try_from(bytes).map_err(AudioError::Count)
}

fn interpolate(left: i16, right: i16, phase: u32) -> Result<i16, AudioError> {
    // The validated rate and PCM width bound the numerator; saturating steps
    // keep it total if a future format widens without changing this function.
    let weighted = i64::from(left)
        .saturating_mul(i64::from(PROVIDER_HZ.saturating_sub(phase)))
        .saturating_add(i64::from(right).saturating_mul(i64::from(phase)));
    let normalizer = i64::from(i16::MIN).saturating_neg();
    let divisor = i64::from(PROVIDER_HZ).saturating_mul(normalizer);
    let rounded = weighted
        .saturating_mul(i64::from(i16::MAX))
        .saturating_add(divisor.div_euclid(2))
        .div_euclid(divisor);
    i16::try_from(rounded).map_err(AudioError::SampleValue)
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
        let length = u32::try_from(self.0.len()).map_err(AudioError::Count)?;
        let header = u32::try_from(WAV_HEADER_BYTES.saturating_sub(RIFF_PREFIX_BYTES))
            .map_err(AudioError::Count)?;
        let riff = header.checked_add(length).ok_or(AudioError::WaveTooLong)?;
        output.extend_from_slice(b"RIFF");
        output.extend_from_slice(&riff.to_le_bytes());
        output.extend_from_slice(b"WAVEfmt ");
        output.extend_from_slice(&WAVE_FMT_BYTES.to_le_bytes());
        output.extend_from_slice(&PCM_FORMAT.to_le_bytes());
        output.extend_from_slice(&MONO_CHANNELS.to_le_bytes());
        let width = u16::try_from(SAMPLE_BYTES).map_err(AudioError::Count)?;
        let bits = u16::try_from(i16::BITS).map_err(AudioError::Count)?;
        output.extend_from_slice(&PROVIDER_HZ.to_le_bytes());
        output.extend_from_slice(&PROVIDER_HZ.saturating_mul(u32::from(width)).to_le_bytes());
        output.extend_from_slice(&width.to_le_bytes());
        output.extend_from_slice(&bits.to_le_bytes());
        output.extend_from_slice(b"data");
        output.extend_from_slice(&length.to_le_bytes());
        output.extend_from_slice(&self.0);
        Ok(())
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
    fn new() -> Result<Self, AudioError> {
        let rate = usize::try_from(PROVIDER_HZ).map_err(AudioError::Count)?;
        Ok(Self {
            pcm: Vec::new(),
            peak: 0,
            bytes_per_second: rate.saturating_mul(SAMPLE_BYTES),
        })
    }

    /// Refuse before retaining a chunk that would exceed the segment bound.
    pub(super) fn append(&mut self, chunk: &ResampledPcm) -> Result<(), AudioError> {
        if self.pcm.len().saturating_add(chunk.0.len())
            > self.bytes_per_second.saturating_mul(MAX_SEGMENT_SECONDS)
        {
            return Err(AudioError::SegmentTooLong);
        }
        for pair in chunk.0.chunks_exact(SAMPLE_BYTES) {
            let pair = <[u8; SAMPLE_BYTES]>::try_from(pair).map_err(AudioError::SampleWidth)?;
            self.peak = self.peak.max(i16::from_le_bytes(pair).unsigned_abs());
        }
        self.pcm.extend_from_slice(&chunk.0);
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
