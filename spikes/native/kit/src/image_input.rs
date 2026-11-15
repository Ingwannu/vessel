//! Normalize native image intake using the image codec already linked by GPUI.
//! Byte signatures choose the decoder; names and clipboard MIME are not proof.
//! Strict side bounds also apply to preserved PNG/JPEG before GPUI sees them.
use image::{DynamicImage, ImageFormat, ImageReader, Limits};
use std::io::{Cursor, Seek, SeekFrom, Write};

/// A static first frame cannot exceed 4096 by 4096 pixels.
pub const MAX_SIDE: u32 = 4096;
/// Decoder allocation budget; width/height are additionally strict codec limits.
pub const MAX_DECODED: u64 = 64 * 1024 * 1024;

/// The caller supplies its existing node byte policy, rather than another limit.
#[derive(Debug, Clone, Copy)]
pub struct InputLimits {
    /// Largest encoded source admitted for conversion.
    pub source: u64,
    /// Largest normalized image the node accepts.
    pub result: u64,
}
/// The two formats retained by Paseo's picked-image normalizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// PNG, also the conversion format for other supported codecs.
    Png,
    /// JPEG keeps its original encoded bytes after bounded validation.
    Jpeg,
}
impl Format {
    /// The MIME declaration always matches the inspected signature.
    #[must_use]
    pub const fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
        }
    }
    /// A native generated image filename uses this extension.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
        }
    }
}
/// Validated encoded bytes; no decoder or device resource survives normalization.
pub struct Normalized {
    /// Encoded PNG/JPEG, bounded by the caller's result limit.
    pub bytes: Vec<u8>,
    /// Its inspected or normalized format.
    pub format: Format,
}
impl std::fmt::Debug for Normalized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Normalized")
            .field("bytes", &self.bytes.len())
            .field("format", &self.format)
            .finish()
    }
}
/// Intake refuses before creating a managed file or displaying a thumbnail.
#[derive(Debug, thiserror::Error)]
pub enum ImageProblem {
    /// Empty or excessive encoded input/output never reaches the decoder/store.
    #[error("image bytes exceed the attachment policy")]
    Size,
    /// Only supported raster decoders may inspect an image.
    #[error("unsupported image encoding")]
    Format,
    /// Byte-only cursor errors carry no source path or contents.
    #[error("image bytes could not be inspected")]
    Inspect(#[source] std::io::Error),
    /// Codec errors remain typed; no raw bytes are recorded.
    #[error("image could not be decoded or normalized")]
    Codec(#[from] image::ImageError),
}

/// Preserve validated PNG/JPEG; normalize other supported rasters to one PNG frame.
///
/// # Errors
/// Refuses invalid signatures, decode/dimension limits, or excessive output.
pub fn normalize(bytes: Vec<u8>, policy: InputLimits) -> Result<Normalized, ImageProblem> {
    if bytes.is_empty() || u64::try_from(bytes.len()).unwrap_or(u64::MAX) > policy.source {
        return Err(ImageProblem::Size);
    }
    let mut reader = ImageReader::new(Cursor::new(bytes.as_slice()))
        .with_guessed_format()
        .map_err(ImageProblem::Inspect)?;
    let format = reader.format().ok_or(ImageProblem::Format)?;
    if !matches!(
        format,
        ImageFormat::Png
            | ImageFormat::Jpeg
            | ImageFormat::Gif
            | ImageFormat::WebP
            | ImageFormat::Bmp
            | ImageFormat::Tiff
    ) {
        return Err(ImageProblem::Format);
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(MAX_DECODED);
    reader.limits(limits);
    let decoded = reader.decode()?;
    let preserved = match format {
        ImageFormat::Png
            if !image::codecs::png::PngDecoder::new(Cursor::new(bytes.as_slice()))?
                .is_apng()? =>
        {
            Some(Format::Png)
        }
        ImageFormat::Jpeg => Some(Format::Jpeg),
        _ => None,
    };
    if let Some(format) = preserved {
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > policy.result {
            return Err(ImageProblem::Size);
        }
        return Ok(Normalized { bytes, format });
    }
    let mut output = Output {
        cursor: Cursor::new(Vec::new()),
        limit: policy.result,
    };
    DynamicImage::ImageRgba8(decoded.into_rgba8()).write_to(&mut output, ImageFormat::Png)?;
    Ok(Normalized {
        bytes: output.cursor.into_inner(),
        format: Format::Png,
    })
}

/// The encoder cannot grow a temporary Vec past the node's encoded-image limit.
struct Output {
    cursor: Cursor<Vec<u8>>,
    limit: u64,
}
impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .cursor
            .position()
            .checked_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
            .is_none_or(|end| end > self.limit)
        {
            return Err(std::io::Error::other(
                "normalized image exceeds the attachment limit",
            ));
        }
        self.cursor.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.cursor.flush()
    }
}
impl Seek for Output {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        let previous = self.cursor.position();
        let next = self.cursor.seek(position)?;
        if next > self.limit {
            self.cursor.set_position(previous);
            return Err(std::io::Error::other(
                "normalized image seek exceeds the attachment limit",
            ));
        }
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const POLICY: InputLimits = InputLimits {
        source: 1024 * 1024,
        result: 1024 * 1024,
    };
    fn encoded(format: ImageFormat) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::new_rgba8(2, 2)
            .write_to(&mut bytes, format)
            .unwrap();
        bytes.into_inner()
    }
    #[test]
    fn png_is_validated_and_preserved_while_bmp_is_converted() {
        let original = encoded(ImageFormat::Png);
        let normalized = normalize(original.clone(), POLICY).unwrap();
        assert_eq!(normalized.bytes, original);
        assert_eq!(normalized.format, Format::Png);
        let normalized = normalize(encoded(ImageFormat::Bmp), POLICY).unwrap();
        assert_eq!(
            image::guess_format(&normalized.bytes).unwrap(),
            ImageFormat::Png
        );
    }
    #[test]
    fn invalid_oversized_and_encoder_overflow_refuse_without_a_partial_result() {
        assert!(normalize(vec![1, 2, 3], POLICY).is_err());
        assert!(matches!(
            normalize(
                vec![1, 2, 3],
                InputLimits {
                    source: 2,
                    ..POLICY
                }
            ),
            Err(ImageProblem::Size)
        ));
        assert!(
            normalize(
                encoded(ImageFormat::Bmp),
                InputLimits {
                    result: 1,
                    ..POLICY
                }
            )
            .is_err()
        );
        let mut huge = Cursor::new(Vec::new());
        DynamicImage::new_rgba8(MAX_SIDE.saturating_add(1), 1)
            .write_to(&mut huge, ImageFormat::Png)
            .unwrap();
        assert!(normalize(huge.into_inner(), POLICY).is_err());
    }
}
