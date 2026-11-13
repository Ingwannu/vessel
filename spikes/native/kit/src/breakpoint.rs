//! The widths at which the layout changes (ADR 0036: one experience from
//! a phone to a desktop): below [`PHONE`] the sidebar becomes a sheet and
//! the title bar's statuses collapse to icons; below [`INSPECTOR`] the
//! inspector becomes a sheet. The page uses the same widths, so a screen
//! changes at the same size on every surface.

use gpui::{Window, px};

/// Minimum interactive target on touch, as the kit's phone controls.
pub const TOUCH_TARGET: f32 = 44.0;

/// Below this many points wide, a phone's layout.
pub const PHONE: f32 = 768.0;

/// Below this many points wide, the inspector is a sheet.
pub const INSPECTOR: f32 = 1280.0;

/// Whether `window` is a phone's width.
#[must_use]
pub fn phone(window: &Window) -> bool {
    window.viewport_size().width < px(PHONE)
}
