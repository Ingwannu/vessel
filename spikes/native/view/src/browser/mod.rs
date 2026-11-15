//! Browser panel rules without a page engine, network or toolkit (ADR 0078).
//!
//! Panes and node targets retain their owners. This module owns only the
//! toolbar's eligibility and the device viewport a pane requests; no fixture
//! may turn an unavailable browser into a visible interactive page.

mod navigation;
mod viewport;

pub use navigation::{History, LoadState, Navigation, NavigationAction};
pub use viewport::{CssPixels, DevicePreset, FixedViewport, Viewport};
