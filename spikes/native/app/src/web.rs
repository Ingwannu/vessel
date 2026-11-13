//! The app in a browser: the same shell and screens, on GPUI's web
//! platform (WebAssembly, drawn with WebGPU or WebGL2 into one canvas).
//!
//! [Decision Log]
//! - Purpose: the browser track (docs/plans/backend-m12-browser.md) asks
//!   whether the one UI crate can be the web client too (ADR 0036). This
//!   is its entry point: nothing of a screen differs.
//! - Constraint: a page has one canvas and one "window" (GPUI's web
//!   platform refuses a second), no environment variables and no
//!   process; the scheme follows the page's `prefers-color-scheme`.
//! - Alternatives: the multi-threaded web platform (GPUI's default):
//!   needs a nightly compiler rebuilding std with atomics and a page
//!   served cross-origin isolated (COOP and COEP), measured separately;
//!   a `#[wasm_bindgen(start)]` library instead of `main` (another crate
//!   type for the same thing: `wasm-bindgen` runs a binary's `main`).
//! - Choice: single-threaded, `main` as the start, the window's options
//!   the desktop's (a page ignores its bounds and decorations).
//! - Cost: background work (text shaping of a large conversation) runs on
//!   the page's one thread between frames.

use gpui::{AppContext as _, Bounds, px, size};
use vessel_app_kit::SchemeChoice;
use vessel_app_view::Lang;

use crate::{LABEL, shell};

/// Start the app in the page that loaded it.
pub(crate) fn main() {
    gpui_platform::web_init();
    // The page's language (`navigator.languages`), as the desktop takes the
    // system's.
    let lang = sys_locale::get_locale().map_or(Lang::En, |tag| Lang::from_tag(&tag));
    gpui_platform::single_threaded_web()
        .with_assets(vessel_app_kit::Assets)
        .run(move |cx| {
            if let Err(error) = vessel_app_kit::init(cx, SchemeChoice::System) {
                tracing::error!(%error, "the kit could not start");
                return;
            }
            // The canvas fills the page whatever the bounds say; the
            // desktop's options are kept so the window is described once.
            let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
            let options = vessel_app_kit::chrome::window_options(format!("Vessel {LABEL}"), bounds);
            let opened = cx.open_window(options, |window, cx| {
                window.on_next_frame(|_window, _cx| mark_first_frame());
                let shell = cx.new(|cx| shell::Shell::new(lang.text(), window, cx));
                vessel_app_kit::root(shell, window, cx)
            });
            if let Err(error) = opened {
                tracing::error!(%error, "the window could not open");
            }
        });
}

/// `vessel-first-frame` on the page's performance timeline, so time to
/// the first frame is read from the page as the page's own numbers are
/// (docs/PERFORMANCE.md; docs/plans/backend-m12-browser.md, question 5).
fn mark_first_frame() {
    if let Some(performance) = web_sys::window().and_then(|window| window.performance())
        && let Err(error) = performance.mark("vessel-first-frame")
    {
        tracing::warn!(?error, "the first frame could not be marked");
    }
}
