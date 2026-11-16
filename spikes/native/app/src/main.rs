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
//! size; `VESSEL_APP_TITLEBAR=system` the system's title bar;
//! `VESSEL_APP_NO_UPDATE` no updates for one run; `RUST_LOG` the log
//! filter (stderr and the log file, `vessel_app_log::dir()`).

mod agent_mark;
mod areas;
pub mod badge;
mod brand;
mod chrome;
#[cfg(not(target_arch = "wasm32"))]
mod host_windows;
mod language;
#[cfg(not(target_arch = "wasm32"))]
mod motion;
mod region;
mod routes;
mod shell;
mod syntax_kind;
mod unsent;
// A browser build does not update itself: `update_off.rs` stands in.
#[cfg_attr(target_arch = "wasm32", path = "update_off.rs")]
mod update;
mod update_words;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
use gpui::{AppContext as _, Bounds, px, size};
#[cfg(not(target_arch = "wasm32"))]
use vessel_app_kit::{Scheme, SchemeChoice};

/// The build's label (`VESSEL_BUILD_LABEL` when it was built, `dev`
/// otherwise): shown in the window's title and the sidebar's foot, and
/// sent to nodes as this client's version.
pub(crate) const LABEL: &str = match option_env!("VESSEL_BUILD_LABEL") {
    Some(label) => label,
    None => "dev",
};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // stderr as before, and a file the owner can send (the release
    // track's `vessel-app-log`: rotated, panics included, the About
    // screen's folder is `vessel_app_log::dir()`).
    let _log = vessel_app_log::init(LABEL);
    let choice = scheme_choice();
    let (width, height) = window_size();
    let prefs = vessel_app_link::Prefs::load();
    let language = language::Language::new();
    let words = match language.prepare(prefs.settings.language) {
        Ok(words) => words,
        Err(error) => {
            tracing::error!(%error, "the embedded translations could not start");
            return;
        }
    };
    gpui_platform::application()
        .with_assets(vessel_app_kit::Assets)
        .run(move |cx| {
            if let Err(error) = vessel_app_kit::init(cx, choice) {
                tracing::error!(%error, "the kit could not start");
                cx.quit();
                return;
            }
            motion::follow(cx);
            // The kept Animations setting, before the first frame.
            motion::animations(prefs.settings.appearance.animations.on(), cx);
            let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
            let options = vessel_app_kit::chrome::window_options(format!("Vessel {LABEL}"), bounds);
            let opened = cx.open_window(options, |window, cx| {
                motion::note_renderer(window, cx);
                let shell = cx.new(|cx| shell::Shell::new(prefs, language, words, window, cx));
                host_windows::register(&shell, window, cx);
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

#[cfg(not(target_arch = "wasm32"))]
fn scheme_choice() -> SchemeChoice {
    match std::env::var("VESSEL_APP_THEME").as_deref() {
        Ok("light") => SchemeChoice::Fixed(Scheme::Light),
        Ok("dark") => SchemeChoice::Fixed(Scheme::Dark),
        Ok(_) | Err(_) => SchemeChoice::System,
    }
}

#[cfg(not(target_arch = "wasm32"))]
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

/// In a browser: GPUI's web platform, one canvas (`web.rs`).
#[cfg(target_arch = "wasm32")]
fn main() {
    web::main();
}

/// Visual-only fixture: run the built test executable under a private Xvfb,
/// with isolated `VESSEL_CONFIG_DIR` and `VESSEL_TEST_PAIR_LINK_FILE`. The renderer
/// and offer reader are real; the browser transport is not exercised here.
#[cfg(all(test, target_os = "linux"))]
#[test]
#[ignore = "requires a private display, config and fake-fleet offer"]
fn pairing_browser_words_capture() {
    assert!(
        std::env::var_os("VESSEL_CONFIG_DIR").is_some_and(|dir| !dir.is_empty()),
        "use an isolated capture config"
    );
    let offer_file = std::env::var_os("VESSEL_TEST_PAIR_LINK_FILE").unwrap();
    let pasted = std::fs::read_to_string(offer_file).unwrap();
    let prefs = vessel_app_link::Prefs::load();
    let language = language::Language::new();
    let words = language.prepare(prefs.settings.language).unwrap();
    let (width, height) = window_size();
    gpui_platform::application()
        .with_assets(vessel_app_kit::Assets)
        .run(move |cx| {
            vessel_app_kit::init(cx, scheme_choice()).unwrap();
            cx.on_window_closed(|cx, _closed| cx.quit()).detach();
            let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
            let options = vessel_app_kit::chrome::window_options(format!("Vessel {LABEL}"), bounds);
            let _window = cx
                .open_window(options, |window, cx| {
                    let shell = cx.new(|cx| {
                        let mut shell = shell::Shell::new(prefs, language, words, window, cx);
                        areas::pair::capture::browser_unsupported(&mut shell, &pasted, window, cx);
                        shell
                    });
                    vessel_app_kit::root(shell, window, cx)
                })
                .unwrap();
            cx.activate(true);
        });
}

/// Visual fixture; the installed app contains no terminal-state override.
#[cfg(all(test, target_os = "linux"))]
#[test]
#[ignore = "requires a private display and a disposable fake fleet"]
fn terminal_states_capture() {
    areas::terminal::capture::run();
}
