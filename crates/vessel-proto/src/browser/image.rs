//! Bounded PNG transport and geometry, before any decoder allocates pixel storage.
//!
//! Recognizing a header is not proof of paint or a complete decodable image.
//! The native codec also enforces its allocation limit and returns a typed
//! failure for corrupt pixels. These transport checks alone are never a paint/engine-readiness claim.

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use super::MAX_VIEWPORT_SIDE;

/// Cap base64-decoded PNG bytes independently from the raster allocation.
pub const MAX_SCREENSHOT_BYTES: usize = 8 * 1024 * 1024;

/// Bound the decoded RGBA surface separately from compressed PNG transport.
pub const MAX_RGBA_BYTES: u64 = 32 * 1024 * 1024;
/// The renderer stores four bytes per RGBA pixel.
const RGBA_BYTES_PER_PIXEL: u64 = 4;
const MAX_ENCODED_BYTES: usize = MAX_SCREENSHOT_BYTES.div_ceil(3).saturating_mul(4);
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const PNG_DIMENSION_WORD: usize = 4;
const PNG_WIDTH_OFFSET: usize = 16;
const PNG_HEIGHT_OFFSET: usize = 20;
const PNG_CHUNK_KIND: std::ops::Range<usize> = 12..16;

/// Content errors never include a page-selected payload or filesystem path.
#[derive(Debug, thiserror::Error)]
pub enum InvalidBrowserImage {
    /// Apply before and after base64 decoding.
    #[error("screenshot exceeds {MAX_SCREENSHOT_BYTES} PNG bytes")]
    EncodedSize,
    /// Validate geometry before a codec can allocate a large raster.
    #[error("screenshot geometry exceeds the {MAX_RGBA_BYTES} RGBA byte budget")]
    Geometry,
    /// A transport marker alone does not identify an image's dimensions.
    #[error("screenshot is missing a PNG image header")]
    Header,
    /// Preserve the decoder cause without printing image data.
    #[error("screenshot encoding is invalid")]
    Encoding(#[source] base64::DecodeError),
}

/// Physical image pixels are not the browser's CSS-pixel viewport dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImagePixels(u32);
impl ImagePixels {
    /// A renderer/metadata adapter explicitly converts this unit.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Geometry was checked against side and total-allocation bounds together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageSize {
    width: ImagePixels,
    height: ImagePixels,
}
impl ImageSize {
    fn checked(width: u32, height: u32) -> Result<Self, InvalidBrowserImage> {
        if width == 0 || height == 0 || width > MAX_VIEWPORT_SIDE || height > MAX_VIEWPORT_SIDE {
            return Err(InvalidBrowserImage::Geometry);
        }
        // Saturation treats overflow as too large, never a small wrapped image.
        let rgba = u64::from(width)
            .saturating_mul(u64::from(height))
            .saturating_mul(RGBA_BYTES_PER_PIXEL);
        if rgba > MAX_RGBA_BYTES {
            return Err(InvalidBrowserImage::Geometry);
        }
        Ok(Self {
            width: ImagePixels(width),
            height: ImagePixels(height),
        })
    }
    /// Physical width comes from the PNG header, not caller-supplied metadata.
    #[must_use]
    pub const fn width(self) -> ImagePixels {
        self.width
    }
    /// Physical height comes from the same checked header.
    #[must_use]
    pub const fn height(self) -> ImagePixels {
        self.height
    }
}

/// Opaque image bytes with bounded geometry; full codec validation remains at display.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ScreenshotPng {
    bytes: Bytes,
    size: ImageSize,
}
impl ScreenshotPng {
    /// Check transport/geometry before retaining a browser-provided image.
    ///
    /// # Errors
    /// Oversized/unknown headers and oversized rasters are refused before decoding pixels.
    pub fn from_png(bytes: Bytes) -> Result<Self, InvalidBrowserImage> {
        if bytes.len() > MAX_SCREENSHOT_BYTES {
            return Err(InvalidBrowserImage::EncodedSize);
        }
        if !bytes.starts_with(PNG_SIGNATURE)
            || bytes.get(PNG_CHUNK_KIND) != Some(b"IHDR".as_slice())
        {
            return Err(InvalidBrowserImage::Header);
        }
        let width = word(&bytes, PNG_WIDTH_OFFSET).ok_or(InvalidBrowserImage::Header)?;
        let height = word(&bytes, PNG_HEIGHT_OFFSET).ok_or(InvalidBrowserImage::Header)?;
        let size = ImageSize::checked(width, height)?;
        Ok(Self { bytes, size })
    }
    /// Explicit image channels may borrow the PNG, not diagnostics or layout persistence.
    #[must_use]
    pub fn bytes(&self) -> &Bytes {
        &self.bytes
    }
    /// Untrusted metadata cannot override these header-derived dimensions.
    #[must_use]
    pub const fn size(&self) -> ImageSize {
        self.size
    }
}
fn word(bytes: &[u8], start: usize) -> Option<u32> {
    let end = start.checked_add(PNG_DIMENSION_WORD)?;
    let word = bytes.get(start..end)?;
    let array = <[u8; PNG_DIMENSION_WORD]>::try_from(word).ok()?;
    Some(u32::from_be_bytes(array))
}
impl TryFrom<String> for ScreenshotPng {
    type Error = InvalidBrowserImage;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > MAX_ENCODED_BYTES {
            return Err(InvalidBrowserImage::EncodedSize);
        }
        let bytes = STANDARD
            .decode(value)
            .map_err(InvalidBrowserImage::Encoding)?;
        Self::from_png(Bytes::from(bytes))
    }
}
impl From<ScreenshotPng> for String {
    fn from(value: ScreenshotPng) -> Self {
        STANDARD.encode(value.bytes())
    }
}
impl fmt::Debug for ScreenshotPng {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ScreenshotPng([redacted])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn header(width: u32, height: u32) -> Bytes {
        let mut bytes = vec![0; PNG_HEIGHT_OFFSET.saturating_add(PNG_DIMENSION_WORD)];
        for (out, value) in bytes.iter_mut().zip(PNG_SIGNATURE) {
            *out = *value;
        }
        for (out, value) in bytes.iter_mut().skip(PNG_CHUNK_KIND.start).zip(b"IHDR") {
            *out = *value;
        }
        for (out, value) in bytes
            .iter_mut()
            .skip(PNG_WIDTH_OFFSET)
            .zip(width.to_be_bytes())
        {
            *out = value;
        }
        for (out, value) in bytes
            .iter_mut()
            .skip(PNG_HEIGHT_OFFSET)
            .zip(height.to_be_bytes())
        {
            *out = value;
        }
        Bytes::from(bytes)
    }
    #[test]
    fn a_small_compressed_header_cannot_claim_an_unbounded_raster() {
        assert!(matches!(
            ScreenshotPng::from_png(header(MAX_VIEWPORT_SIDE, MAX_VIEWPORT_SIDE)),
            Err(InvalidBrowserImage::Geometry)
        ));
        assert!(matches!(
            ScreenshotPng::from_png(header(0, 1)),
            Err(InvalidBrowserImage::Geometry)
        ));
        assert!(ScreenshotPng::from_png(Bytes::from_static(PNG_SIGNATURE)).is_err());
        let image = ScreenshotPng::from_png(header(1, 1)).unwrap();
        assert_eq!(image.size().width().get(), 1);
        // A header fixture validates geometry only; it is never reported as a painted page.
        assert_eq!(
            serde_json::from_str::<ScreenshotPng>(&serde_json::to_string(&image).unwrap()).unwrap(),
            image
        );
    }
}
