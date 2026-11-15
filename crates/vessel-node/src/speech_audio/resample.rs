//! Integer phase and one carry sample preserve chunking invariance.

use super::{CodecError, PROVIDER_RATE, Pcm16, Rate, ResampledPcm, SAMPLE_BYTES, Storage, reserve};

/// Conversion state holds one sample, never an earlier capture buffer.
pub(crate) struct Resampler {
    input: Rate,
    phase: u32,
    left: Option<i16>,
}
impl Resampler {
    /// A parsed rate selects the ratio; providers share one typed output rate.
    pub(crate) fn new(input: Rate) -> Self {
        Self {
            input,
            phase: 0,
            left: None,
        }
    }
    /// Keep borrowed bypass output and bounded converted output payload-redacted.
    pub(crate) fn push<'a>(&mut self, pcm: Pcm16<'a>) -> Result<ResampledPcm<'a>, CodecError> {
        if self.input == PROVIDER_RATE {
            return Ok(ResampledPcm(Storage::Borrowed(pcm)));
        }
        let mut output = Vec::new();
        reserve(&mut output, capacity(pcm, self.input)?)?;
        for sample in pcm.as_slice().chunks_exact(SAMPLE_BYTES) {
            let pair = <[u8; SAMPLE_BYTES]>::try_from(sample).map_err(CodecError::SampleWidth)?;
            let right = i16::from_le_bytes(pair);
            if let Some(left) = self.left {
                while self.phase < PROVIDER_RATE.get() {
                    output.extend_from_slice(&interpolate(left, right, self.phase)?.to_le_bytes());
                    // Parsed rates bound this phase well below u32; saturation
                    // makes future widened formats refuse rather than wrap.
                    self.phase = self.phase.saturating_add(self.input.get());
                }
                self.phase = self.phase.saturating_sub(PROVIDER_RATE.get());
            }
            self.left = Some(right);
        }
        Ok(ResampledPcm(Storage::Converted(output.into())))
    }
}
fn capacity(pcm: Pcm16<'_>, rate: Rate) -> Result<usize, CodecError> {
    let samples =
        u64::try_from(pcm.as_slice().len().div_euclid(SAMPLE_BYTES)).map_err(CodecError::Count)?;
    let width = u64::try_from(SAMPLE_BYTES).map_err(CodecError::Count)?;
    let bytes = samples
        .saturating_mul(u64::from(PROVIDER_RATE.get()))
        .div_ceil(u64::from(rate.get()))
        .saturating_mul(width);
    usize::try_from(bytes).map_err(CodecError::Count)
}
fn interpolate(left: i16, right: i16, phase: u32) -> Result<i16, CodecError> {
    // This is floor(x + .5), including negative samples: JavaScript Math.round.
    // Validated rates bound weighted * i16::MAX far inside i64.
    let weighted = i64::from(left)
        .saturating_mul(i64::from(PROVIDER_RATE.get().saturating_sub(phase)))
        .saturating_add(i64::from(right).saturating_mul(i64::from(phase)));
    let normalizer = i64::from(i16::MIN).saturating_neg();
    let divisor = i64::from(PROVIDER_RATE.get()).saturating_mul(normalizer);
    let rounded = weighted
        .saturating_mul(i64::from(i16::MAX))
        .saturating_add(divisor.div_euclid(2))
        .div_euclid(divisor);
    i16::try_from(rounded).map_err(CodecError::SampleValue)
}
