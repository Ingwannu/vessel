//! The widths at which the layout changes (ADR 0036: one experience from
//! a phone to a desktop): below [`PHONE`] the app takes its phone form;
//! below [`INSPECTOR`] the inspector becomes a sheet.
//!
//! [Decision Log]
//! - Purpose and intent: one compact width, as Paseo has. Paseo's compact is
//!   below its `md` breakpoint, 600 (`constants/layout.ts`
//!   `isCompactBreakpoint`, `styles/unistyles.ts`), and every sheet form
//!   switches there: `AdaptiveModalSheet`, a menu's `compactMode="sheet"`,
//!   the combobox's sheet.
//! - Existing implementation and constraints: the kit's phone width was 768
//!   (the page's sidebar fold), while the layout's Compact
//!   (`vessel_app_view::layout::COMPACT_LAYOUT_WIDTH`), the sidebar's sheet
//!   and the pickers' touch sizes (`picker::COMPACT`) were already 600. So
//!   between 600 and 768 the wide bar and strip were drawn while dialogs and
//!   titled menus came up as phone sheets (the app reviewer on TRACKER 798).
//! - Alternatives considered: a second condition on each sheet (one more
//!   width to keep in step, the band left wrong everywhere else).
//! - Chosen approach: [`PHONE`] is 600, `picker::COMPACT` is this constant,
//!   and the app holds it equal to the layout's Compact at compile time (the
//!   view cannot depend on the kit). The coordinator's ruling, 2026-11-18.
//! - Pros, cons and impact: between 600 and 768 dialogs are centred, menus
//!   are popovers and controls are pointer-sized, as in Paseo. The page
//!   (the browser build's own shell) still folds its sidebar at 768.

use gpui::{Window, px};

/// Minimum interactive target on touch, as the kit's phone controls.
pub const TOUCH_TARGET: f32 = 44.0;

/// Below this many points wide, a phone's form: Paseo's one compact width
/// (below `md`, 600). Keep it equal to the layout's Compact width.
pub const PHONE: f32 = 600.0;

/// Below this many points wide, the inspector is a sheet.
pub const INSPECTOR: f32 = 1280.0;

/// Whether `window` is a phone's width.
#[must_use]
pub fn phone(window: &Window) -> bool {
    window.viewport_size().width < px(PHONE)
}
