//! `vessel-app`: Vessel's native app (milestone 12's first builds).
//!
//! One window: the sidebar with the machines in the node book and their
//! sessions, the board's lanes, and a session's conversation with its
//! composer; a pairing form when the book is empty or a node is to be
//! added. It reads the same node book, token files and device
//! credentials as the `vessel` CLI (`VESSEL_CONFIG_DIR` points it at
//! another book, as it does the CLI).
//!
//! Environment, for testing and captures: `VESSEL_APP_THEME=light|dark`
//! pins the scheme (otherwise the system's); `VESSEL_APP_LANG=ko|en` the
//! language (otherwise the system's); `VESSEL_APP_SIZE=WxH` the window's
//! size; `RUST_LOG` the log filter.

mod areas;
mod chrome;
mod routes;
mod shell;

use gpui::{AppContext as _, Bounds, px, size};
use vessel_app_kit::{Scheme, SchemeChoice};
use vessel_app_view::Lang;

/// The build's label (`VESSEL_BUILD_LABEL` when it was built, `dev`
/// otherwise): shown in the window's title and the sidebar's foot, and
/// sent to nodes as this client's version.
pub(crate) const LABEL: &str = match option_env!("VESSEL_BUILD_LABEL") {
    Some(label) => label,
    None => "dev",
};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
    let choice = match std::env::var("VESSEL_APP_THEME").as_deref() {
        Ok("light") => SchemeChoice::Fixed(Scheme::Light),
        Ok("dark") => SchemeChoice::Fixed(Scheme::Dark),
        Ok(_) | Err(_) => SchemeChoice::System,
    };
    let (width, height) = window_size();
    let lang = language();
    gpui_platform::application()
        .with_assets(vessel_app_kit::Assets)
        .run(move |cx| {
            if let Err(error) = vessel_app_kit::init(cx, choice) {
                tracing::error!(%error, "the kit could not start");
                cx.quit();
                return;
            }
            let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
            let options = vessel_app_kit::chrome::window_options(format!("Vessel {LABEL}"), bounds);
            let opened = cx.open_window(options, |window, cx| {
                let shell = cx.new(|cx| shell::Shell::new(lang.text(), window, cx));
                vessel_app_kit::root(shell, window, cx)
            });
            match opened {
                Ok(_window) => cx.activate(true),
                Err(error) => {
                    tracing::error!(%error, "the window could not open");
                    cx.quit();
                }
            }
        });
}

/// The app's language: `VESSEL_APP_LANG` (`ko`, `en`), else the system's
/// (Korean for any `ko` locale, English otherwise).
fn language() -> Lang {
    std::env::var("VESSEL_APP_LANG")
        .ok()
        .or_else(sys_locale::get_locale)
        .map_or(Lang::En, |tag| Lang::from_tag(&tag))
}

/// The window's size: `VESSEL_APP_SIZE=WxH`, else 1280 by 800.
fn window_size() -> (f32, f32) {
    std::env::var("VESSEL_APP_SIZE")
        .ok()
        .and_then(|text| {
            let (width, height) = text.split_once('x')?;
            Some((width.parse().ok()?, height.parse().ok()?))
        })
        .unwrap_or((1280.0, 800.0))
}
