//! Compose the single image decoder with view geometry, without frame authority.
//!
//! Decoded dimensions must match metadata before a CPU image can carry a map.
//! The real owner must separately fence node/resource/revision, run decoding off
//! the UI executor and bound retained images; this staged model does none of those.

use std::{fmt, sync::Arc};
use vessel_app_kit::image_input::{ImageProblem, screenshot::decode_png};
use vessel_app_view::browser::{InvalidSurface, PageSize, PaneSize, RasterSize, SurfaceMap};

struct PreparedFrame {
    image: Arc<gpui::RenderImage>,
    raster: RasterSize,
    page: PageSize,
}

#[derive(Debug)]
enum PreparationError {
    Image(ImageProblem),
    Geometry(InvalidSurface),
    RasterMismatch,
}
impl fmt::Display for PreparationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Image(source) => fmt::Display::fmt(source, f),
            Self::Geometry(source) => fmt::Display::fmt(source, f),
            Self::RasterMismatch => {
                f.write_str("browser decoded raster differs from frame metadata")
            }
        }
    }
}
impl std::error::Error for PreparationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Image(source) => Some(source),
            Self::Geometry(source) => Some(source),
            Self::RasterMismatch => None,
        }
    }
}

impl PreparedFrame {
    fn decode(
        bytes: &[u8],
        expected: RasterSize,
        page: PageSize,
    ) -> Result<Self, PreparationError> {
        let decoded = decode_png(bytes).map_err(PreparationError::Image)?;
        let size = decoded.size();
        let width = u32::try_from(i32::from(size.width))
            .map_err(|_| PreparationError::Geometry(InvalidSurface))?;
        let height = u32::try_from(i32::from(size.height))
            .map_err(|_| PreparationError::Geometry(InvalidSurface))?;
        let raster = RasterSize::new(width, height).map_err(PreparationError::Geometry)?;
        if raster != expected {
            return Err(PreparationError::RasterMismatch);
        }
        Ok(Self {
            image: decoded.into_render_image(),
            raster,
            page,
        })
    }

    fn surface(&self, pane: PaneSize) -> Result<SurfaceMap, InvalidSurface> {
        SurfaceMap::fit(self.raster, self.page, pane)
    }
}

#[cfg(test)]
mod tests;
