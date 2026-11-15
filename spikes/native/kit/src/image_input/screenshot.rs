//! PNG screenshot pixels use the existing bounded raster decoder, without I/O.
//!
//! Raster dimensions are independent of the browser's CSS viewport. The browser
//! policy is explicit here so admitting a wide screenshot cannot relax the
//! attachment normalizer's 4096-pixel side bound.
use super::{ImageProblem, decode};
use std::sync::Arc;

/// Largest encoded screenshot admitted before header inspection.
pub const MAX_ENCODED_BYTES: u64 = 8 * 1024 * 1024;
/// Largest raster width or height; CSS dimensions are not used as proof.
pub const MAX_SIDE: u32 = 8192;
/// Largest final RGBA pixel buffer, checked before pixel decode.
pub const MAX_RGBA_BYTES: u64 = 32 * 1024 * 1024;

/// One validated static RGBA frame; encoded bytes and codec state are not kept.
pub struct DecodedRaster {
    pixels: image::RgbaImage,
}
impl std::fmt::Debug for DecodedRaster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodedRaster")
            .field("dimensions", &self.pixels.dimensions())
            .field("bytes", &self.pixels.as_raw().len())
            .finish()
    }
}
impl DecodedRaster {
    /// Actual raster dimensions in device pixels.
    #[must_use]
    pub fn size(&self) -> gpui::Size<gpui::DevicePixels> {
        gpui::size(self.pixels.width().into(), self.pixels.height().into())
    }
    /// Borrow decoded RGBA bytes; no renderer allocation or byte copy.
    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        self.pixels.as_raw()
    }
    /// Transfer the same buffer to GPUI's BGRA frame representation.
    ///
    /// This constructs a CPU render image. GPU upload, visibility and global
    /// cache lifetime remain the caller's responsibility.
    #[must_use]
    pub fn into_render_image(mut self) -> Arc<gpui::RenderImage> {
        for pixel in self.pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
        Arc::new(gpui::RenderImage::new(vec![image::Frame::new(self.pixels)]))
    }
}

/// Decode one PNG screenshot through the attachment owner's existing codec path.
///
/// # Errors
/// Refuses other encodings, invalid pixels, excessive encoded size, dimensions
/// or decoded byte count. Run CPU decoding on a background executor.
pub fn decode_png(bytes: &[u8]) -> Result<DecodedRaster, ImageProblem> {
    let (_, decoded) = decode::read(bytes, decode::Policy::Screenshot)?;
    Ok(DecodedRaster {
        pixels: decoded.into_rgba8(),
    })
}

#[cfg(test)]
mod tests;
