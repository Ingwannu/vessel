//! Shared in-memory little-endian mono PCM16 codecs, staged until a real speech consumer wires them.
//!
//! The first consumer removes the outer test gate. No recording limits, silence,
//! segmentation, prompts, providers or multipart metadata belong here. Producers
//! own their byte budgets; this edge owns alignment, rate conversion and RIFF.

use bytes::Bytes;
use std::fmt;

mod resample;
mod wav;

/// Carry phase across chunks without imposing a recording or segment policy.
pub(crate) use resample::Resampler;
/// Keep one validating rate domain for both speech consumers.
pub(crate) use vessel_proto::dictation::SampleRate as Rate;
/// Append a checked WAV directly to a caller's redacted request body.
pub(crate) use wav::Wav;

/// Speech providers consume the same rate without repeating its numeric value.
pub(crate) const PROVIDER_RATE: Rate = Rate::TRANSCRIPTION;
const SAMPLE_BYTES: usize = size_of::<i16>();

/// Fail without carrying samples while preserving allocation/count/I/O causes.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CodecError {
    #[error("PCM16 requires complete samples")]
    Alignment,
    #[error("PCM16 did not contain complete samples")]
    SampleWidth(#[source] std::array::TryFromSliceError),
    #[error("resampling produced an invalid PCM16 sample")]
    SampleValue(#[source] std::num::TryFromIntError),
    #[error("audio counts exceeded this platform's range")]
    Count(#[source] std::num::TryFromIntError),
    #[error("the WAV exceeds its RIFF size field")]
    WaveTooLong,
    #[error("the audio buffer could not be reserved")]
    Allocation(#[source] std::collections::TryReserveError),
    #[error("the WAV format header could not be encoded")]
    Header(#[source] std::io::Error),
}

/// Borrow aligned samples without copying or converting another feature's PCM type.
/// Empty output is valid here; capture admission and silence belong to consumers.
#[derive(Clone, Copy)]
pub(crate) struct Pcm16<'a>(&'a [u8]);
impl<'a> Pcm16<'a> {
    /// Validate before a stateful converter can observe an incomplete sample.
    pub(crate) fn new(bytes: &'a [u8]) -> Result<Self, CodecError> {
        if !bytes.len().is_multiple_of(SAMPLE_BYTES) {
            return Err(CodecError::Alignment);
        }
        Ok(Self(bytes))
    }
    /// Borrow at the codec edge; the producing feature keeps its own byte bound.
    pub(crate) const fn as_slice(self) -> &'a [u8] {
        self.0
    }
}
impl fmt::Debug for Pcm16<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Pcm16([redacted])")
    }
}

#[derive(Clone)]
enum Storage<'a> {
    Borrowed(Pcm16<'a>),
    Converted(Bytes),
}
/// Distinguish provider-rate samples from the capture-rate input.
#[derive(Clone)]
pub(crate) struct ResampledPcm<'a>(Storage<'a>);
impl ResampledPcm<'_> {
    /// Bypass conversion without a new audio allocation when rates already match.
    pub(crate) fn as_slice(&self) -> &[u8] {
        match &self.0 {
            Storage::Borrowed(pcm) => pcm.as_slice(),
            Storage::Converted(pcm) => pcm,
        }
    }
    /// Preserve the complete-sample invariant at the WAV edge.
    pub(crate) fn as_pcm(&self) -> Pcm16<'_> {
        Pcm16(self.as_slice())
    }
}
impl fmt::Debug for ResampledPcm<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ResampledPcm([redacted])")
    }
}

fn reserve(output: &mut Vec<u8>, additional: usize) -> Result<(), CodecError> {
    output
        .try_reserve(additional)
        .map_err(CodecError::Allocation)
}

#[cfg(test)]
mod tests;
