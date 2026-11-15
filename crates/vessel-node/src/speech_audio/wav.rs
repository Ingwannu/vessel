//! Checked RIFF framing borrows PCM and appends directly to an existing body.

use super::{CodecError, Pcm16, Rate, SAMPLE_BYTES, reserve};
use std::{
    fmt,
    io::{Cursor, Write as _},
};

pub(super) const HEADER_BYTES: usize = 44;
pub(super) const RIFF_PREFIX_BYTES: usize = 8;
pub(super) const FMT_BYTES: u32 = 16;
pub(super) const PCM_FORMAT: u16 = 1;
pub(super) const MONO: u16 = 1;

/// A validated format header and borrowed samples; no second audio allocation.
pub(crate) struct Wav<'a> {
    header: [u8; HEADER_BYTES],
    pcm: Pcm16<'a>,
}
impl<'a> Wav<'a> {
    /// Validate every RIFF/count field before a caller's body can change.
    pub(crate) fn new(rate: Rate, pcm: Pcm16<'a>) -> Result<Self, CodecError> {
        Ok(Self {
            header: header(rate, pcm.as_slice().len())?,
            pcm,
        })
    }
    /// Reserve first so overflow or allocation refusal leaves the prefix intact.
    pub(crate) fn append_to(&self, output: &mut Vec<u8>) -> Result<(), CodecError> {
        let additional = HEADER_BYTES
            .checked_add(self.pcm.as_slice().len())
            .ok_or(CodecError::WaveTooLong)?;
        reserve(output, additional)?;
        output.extend_from_slice(&self.header);
        output.extend_from_slice(self.pcm.as_slice());
        Ok(())
    }
}
impl fmt::Debug for Wav<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Wav([redacted])")
    }
}
fn header(rate: Rate, data: usize) -> Result<[u8; HEADER_BYTES], CodecError> {
    let data = u32::try_from(data).map_err(CodecError::Count)?;
    let prefix =
        u32::try_from(HEADER_BYTES.saturating_sub(RIFF_PREFIX_BYTES)).map_err(CodecError::Count)?;
    let riff = prefix.checked_add(data).ok_or(CodecError::WaveTooLong)?;
    let width = u16::try_from(SAMPLE_BYTES).map_err(CodecError::Count)?;
    let bits = u16::try_from(i16::BITS).map_err(CodecError::Count)?;
    // Parsed rates and complete PCM16 sample width bound this product.
    let byte_rate = rate.get().saturating_mul(u32::from(width));
    let mut output = [0; HEADER_BYTES];
    let mut writer = Cursor::new(output.as_mut_slice());
    for part in [
        b"RIFF".as_slice(),
        &riff.to_le_bytes(),
        b"WAVEfmt ".as_slice(),
        &FMT_BYTES.to_le_bytes(),
        &PCM_FORMAT.to_le_bytes(),
        &MONO.to_le_bytes(),
        &rate.get().to_le_bytes(),
        &byte_rate.to_le_bytes(),
        &width.to_le_bytes(),
        &bits.to_le_bytes(),
        b"data".as_slice(),
        &data.to_le_bytes(),
    ] {
        writer.write_all(part).map_err(CodecError::Header)?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn riff_counts_refuse_before_allocating_a_payload() {
        assert!(header(super::super::PROVIDER_RATE, usize::MAX).is_err());
        let excessive = usize::try_from(u32::MAX).unwrap();
        assert!(matches!(
            header(super::super::PROVIDER_RATE, excessive),
            Err(CodecError::WaveTooLong)
        ));
    }
}
