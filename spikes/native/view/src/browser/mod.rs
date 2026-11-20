//! Browser panel rules without a page engine, network or toolkit (ADR 0078).
//!
//! Panes and node targets retain their owners. This module owns only the
//! toolbar's eligibility and the device viewport a pane requests; no fixture
//! may turn an unavailable browser into a visible interactive page.

mod controls;
mod navigation;
mod surface;
mod viewport;

pub use controls::GeneralPane;
pub use navigation::{History, LoadState, Navigation, NavigationAction};
pub use surface::{
    InvalidSurface, LogicalLength, PageOffset, PagePoint, PageSize, PanePoint, PaneSize,
    RasterSize, SurfaceMap,
};
pub use viewport::{CssPixels, DevicePreset, FixedViewport, Viewport};
