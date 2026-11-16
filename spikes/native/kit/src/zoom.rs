//! A window's zoom: its whole content drawn larger or smaller, as a
//! browser zooms a page, with the display's own scale kept.
//!
//! [Decision Log]
//! - Purpose: Paseo's desktop app zooms its interface with Ctrl+= / Ctrl+-
//!   / Ctrl+0. The app needs the same: everything larger, less of it in the
//!   window, still sharp, and a click landing on what is drawn under it.
//! - Constraint: GPUI takes a window's scale from the platform (the
//!   display's DPI) and changes it only in tests (`set_scale_factor`).
//!   `rem_size` scales only what is sized in rems; the kit sizes in points.
//! - Alternatives:
//!   1. Multiply every kit size by a factor. Every token, every literal
//!      size in the app, and every test of a size would carry it.
//!   2. Size everything in rems. A rewrite of the kit, and GPUI's own
//!      elements (scroll bars, the input base) still use points.
//!   3. The vendored GPUI's per-window zoom (`Window::set_zoom`,
//!      `spikes/release/patches/README.md`, "gpui-pre", change 8): one
//!      multiplier on top of the platform's scale for layout, paint and
//!      hit-testing; input arrives divided by it, and positions the
//!      platform needs (the IME's caret, the window menu, client
//!      decorations) leave multiplied by it.
//! - Choice: 3. Text and images rasterize at the zoomed scale, so they stay
//!   sharp; nothing in the kit or the app changes its sizes.
//! - Cost: one more change to carry in the vendored GPUI core at each pin.
//!   The window's own size and place on the screen (`bounds`,
//!   `window_bounds`, `resize`) stay in the platform's points, so saved
//!   window sizes do not change with the zoom.
//!
//! The app's keys and the remembered zoom are the app's; this module holds
//! the checks of the patch through real events in a test window.

#[cfg(test)]
mod tests;
