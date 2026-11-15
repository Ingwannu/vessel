//! One codec path keeps screenshot projection from bypassing attachment bounds.
use super::{ImageProblem, InputLimits, MAX_DECODED, MAX_SIDE};
use image::{DynamicImage, ImageDecoder as _, ImageFormat, ImageReader, Limits};
use std::io::Cursor;

#[derive(Debug, Clone, Copy)]
pub(super) enum Policy {
    Attachment(InputLimits),
    Screenshot,
}
impl Policy {
    fn limits(&self) -> Limits {
        let mut limits = Limits::default();
        let (side, allocation) = match self {
            Self::Attachment(_) => (MAX_SIDE, MAX_DECODED),
            Self::Screenshot => (
                super::screenshot::MAX_SIDE,
                super::screenshot::MAX_RGBA_BYTES,
            ),
        };
        limits.max_image_width = Some(side);
        limits.max_image_height = Some(side);
        limits.max_alloc = Some(allocation);
        limits
    }
    fn source(&self) -> u64 {
        match self {
            Self::Attachment(policy) => policy.source,
            Self::Screenshot => super::screenshot::MAX_ENCODED_BYTES,
        }
    }
    fn accepts(&self, format: ImageFormat) -> bool {
        match self {
            Self::Attachment(_) => matches!(
                format,
                ImageFormat::Png
                    | ImageFormat::Jpeg
                    | ImageFormat::Gif
                    | ImageFormat::WebP
                    | ImageFormat::Bmp
                    | ImageFormat::Tiff
            ),
            Self::Screenshot => format == ImageFormat::Png,
        }
    }
}

pub(super) fn read(
    bytes: &[u8],
    policy: Policy,
) -> Result<(ImageFormat, DynamicImage), ImageProblem> {
    if bytes.is_empty() || u64::try_from(bytes.len()).unwrap_or(u64::MAX) > policy.source() {
        return Err(ImageProblem::Size);
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(ImageProblem::Inspect)?;
    let format = reader.format().ok_or(ImageProblem::Format)?;
    if !policy.accepts(format) {
        return Err(ImageProblem::Format);
    }
    reader.limits(policy.limits());
    let mut decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    let mut limits = policy.limits();
    // Reserve the decoded native buffer as ImageReader::decode does, and
    // additionally bound the later RGBA projection before any pixel decode.
    let rgba = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(ImageProblem::Size)?;
    if width == 0 || height == 0 || limits.max_alloc.is_some_and(|limit| rgba > limit) {
        return Err(ImageProblem::Size);
    }
    limits.reserve(decoder.total_bytes())?;
    decoder.set_limits(limits)?;
    Ok((format, DynamicImage::from_decoder(decoder)?))
}
