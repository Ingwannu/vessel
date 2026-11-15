//! Pure frame-to-page geometry, without a toolkit, browser id or input authority.
//!
//! [Decision Log]
//! - Purpose: screenshots may have a different physical resolution from CSS,
//!   and a fixed viewport is centered inside a pane. Pane-local clicks must
//!   not turn letterbox space into an unintended page action.
//! - Reuse: the shared finite-number type and preset dimensions; terminal
//!   character-cell rounding cannot express browser subpixel coordinates.
//! - Choice: fit the physical frame without changing its aspect ratio, then
//!   map only the drawn rectangle to its associated CSS viewport. The owner
//!   must carry that same frame's node revision; this model mints no authority.

use std::num::NonZeroU32;

use vessel_proto::conversation::FiniteNumber;

/// The page and raster sizes have distinct units despite sharing positive dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RasterSize {
    width: NonZeroU32,
    height: NonZeroU32,
}

/// The CSS viewport associated with a frame, never inferred from its physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSize {
    width: NonZeroU32,
    height: NonZeroU32,
}

/// Invalid/hidden geometry cannot generate coordinates or imply a painted frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("browser surface geometry must be positive, finite and representable")]
pub struct InvalidSurface;

macro_rules! positive_size {
    ($name:ident) => {
        impl $name {
            /// Parsing metadata here does not authorize an image's memory allocation.
            ///
            /// # Errors
            /// Zero geometry represents no drawable/input surface.
            pub fn new(width: u32, height: u32) -> Result<Self, InvalidSurface> {
                Ok(Self {
                    width: NonZeroU32::new(width).ok_or(InvalidSurface)?,
                    height: NonZeroU32::new(height).ok_or(InvalidSurface)?,
                })
            }
        }
    };
}
positive_size!(RasterSize);
positive_size!(PageSize);

impl TryFrom<super::FixedViewport> for PageSize {
    type Error = InvalidSurface;
    fn try_from(value: super::FixedViewport) -> Result<Self, Self::Error> {
        Self::new(
            u32::from(value.width().get()),
            u32::from(value.height().get()),
        )
    }
}

/// Pane-local logical length; positive and finite before division.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogicalLength(FiniteNumber);
impl LogicalLength {
    /// Parse toolkit layout values once; an unmounted pane's zero is not drawable.
    ///
    /// # Errors
    /// Zero, negative, nonfinite or overflowed values cannot form a viewport.
    pub fn new(value: f64) -> Result<Self, InvalidSurface> {
        if value <= 0.0 {
            return Err(InvalidSurface);
        }
        FiniteNumber::new(value).map(Self).ok_or(InvalidSurface)
    }
    /// The rendering boundary converts this unit to toolkit logical pixels.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0.get()
    }
}

/// Logical pane dimensions differ from physical image pixels and page CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneSize {
    /// Available width after toolbar/content insets.
    pub width: LogicalLength,
    /// Available height after toolbar/content insets.
    pub height: LogicalLength,
}

/// A point from the toolkit's local input event, before any page mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanePoint {
    /// Horizontal pane-local logical pixels.
    pub x: FiniteNumber,
    /// Vertical pane-local logical pixels.
    pub y: FiniteNumber,
}

/// A nonnegative CSS displacement, not an integer device pixel or host cursor position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageOffset(FiniteNumber);
impl PageOffset {
    /// Convert at the typed node input boundary, with its current viewport/revision check.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0.get()
    }
}

/// Mapped page coordinates still require the same target's current node revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PagePoint {
    /// Horizontal CSS pixels.
    pub x: PageOffset,
    /// Vertical CSS pixels.
    pub y: PageOffset,
}

/// Rendering facts are ephemeral and contain no URL, page text or credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceMap {
    left: FiniteNumber,
    top: FiniteNumber,
    width: LogicalLength,
    height: LogicalLength,
    page: PageSize,
}
impl SurfaceMap {
    /// Preserve aspect ratio and center the frame within the available pane.
    ///
    /// # Errors
    /// Invalid/underflowed computed geometry disables input rather than clamping a click.
    pub fn fit(raster: RasterSize, page: PageSize, pane: PaneSize) -> Result<Self, InvalidSurface> {
        let scale = (pane.width.get() / f64::from(raster.width.get()))
            .min(pane.height.get() / f64::from(raster.height.get()));
        let width = LogicalLength::new(f64::from(raster.width.get()) * scale)?;
        let height = LogicalLength::new(f64::from(raster.height.get()) * scale)?;
        let left =
            FiniteNumber::new((pane.width.get() - width.get()) / 2.0).ok_or(InvalidSurface)?;
        let top =
            FiniteNumber::new((pane.height.get() - height.get()) / 2.0).ok_or(InvalidSurface)?;
        if left.get() < 0.0 || top.get() < 0.0 {
            return Err(InvalidSurface);
        }
        Ok(Self {
            left,
            top,
            width,
            height,
            page,
        })
    }

    /// Draw at this pane-local inset; screen coordinates never reach the node.
    #[must_use]
    pub const fn origin(self) -> PanePoint {
        PanePoint {
            x: self.left,
            y: self.top,
        }
    }
    /// The exact logical rectangle used for both painting and input conversion.
    #[must_use]
    pub const fn drawn_size(self) -> PaneSize {
        PaneSize {
            width: self.width,
            height: self.height,
        }
    }

    /// Reject borders/letterboxing; right/bottom edges are outside the page.
    #[must_use]
    pub fn page_point(self, point: PanePoint) -> Option<PagePoint> {
        let local_x = point.x.get() - self.left.get();
        let local_y = point.y.get() - self.top.get();
        if local_x < 0.0
            || local_y < 0.0
            || local_x >= self.width.get()
            || local_y >= self.height.get()
        {
            return None;
        }
        let x = FiniteNumber::new(local_x / self.width.get() * f64::from(self.page.width.get()))?;
        let y = FiniteNumber::new(local_y / self.height.get() * f64::from(self.page.height.get()))?;
        Some(PagePoint {
            x: PageOffset(x),
            y: PageOffset(y),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pane(width: f64, height: f64) -> PaneSize {
        PaneSize {
            width: LogicalLength::new(width).unwrap(),
            height: LogicalLength::new(height).unwrap(),
        }
    }
    fn point(x: f64, y: f64) -> PanePoint {
        PanePoint {
            x: FiniteNumber::new(x).unwrap(),
            y: FiniteNumber::new(y).unwrap(),
        }
    }

    #[test]
    fn retina_frame_uses_css_coordinates_and_excludes_letterboxing() {
        let map = SurfaceMap::fit(
            RasterSize::new(750, 1334).unwrap(),
            PageSize::new(375, 667).unwrap(),
            pane(1000.0, 667.0),
        )
        .unwrap();
        assert_eq!(map.drawn_size(), pane(375.0, 667.0));
        assert_eq!(map.origin(), point(312.5, 0.0));
        assert_eq!(map.page_point(point(500.0, 333.5)).unwrap().x.get(), 187.5);
        assert!(map.page_point(point(312.0, 10.0)).is_none());
        assert!(map.page_point(point(687.5, 10.0)).is_none());
        assert!(map.page_point(point(500.0, 667.0)).is_none());
    }

    #[test]
    fn shrinking_a_pane_maps_the_drawn_frame_without_clicking_the_toolbar_or_borders() {
        let map = SurfaceMap::fit(
            RasterSize::new(1920, 1080).unwrap(),
            PageSize::new(1920, 1080).unwrap(),
            pane(390.0, 400.0),
        )
        .unwrap();
        let center = map.page_point(point(195.0, 200.0)).unwrap();
        assert_eq!(center.x.get(), 960.0);
        assert_eq!(center.y.get(), 540.0);
        assert!(map.page_point(point(100.0, 1.0)).is_none());
        assert!(LogicalLength::new(f64::INFINITY).is_err());
        assert!(LogicalLength::new(0.0).is_err());
        assert!(RasterSize::new(0, 1).is_err());
    }
}
