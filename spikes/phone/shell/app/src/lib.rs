//! Vessel's phone app (milestone 12, track M): a Tauri 2 shell.
//!
//! The Rust core (`vessel-phone-core`) runs Vessel's bridge in this
//! process, on loopback; the one web view shows the page from the bundle
//! embedded in it, and pairs with nodes as the page already does. The web
//! view is given no Tauri command (`tauri.conf.json` grants no
//! capability), navigates nowhere but the app's start page and the
//! bridge's own origin (`window`), and loads nothing remote: the page's
//! own content security policy, sent by the bridge, keeps its requests on
//! that origin.
//!
//! Modules:
//! - `window`: the web view, where it may go, and the build's label shown
//!   on it;
//! - `launch`: a launch of the bridge each time the start page loads;
//! - `logs`: where the app's logs go (logcat on Android).

mod launch;
mod logs;
mod window;

use tauri::{App, Manager as _, RunEvent};
use vessel_phone_core::{AppDirs, Roots};

use crate::launch::Launcher;

/// This build's label (`VESSEL_APP_LABEL` at build time, `dev` without).
pub const LABEL: &str = env!("VESSEL_APP_LABEL");

/// The app's entry point: Android's and iOS's hosts call it (through the
/// symbol Tauri's macro exports), the desktop binary calls it from `main`.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logs::init();
    tracing::info!(label = LABEL, "Vessel phone app starting");
    let built = tauri::Builder::default()
        .setup(|app| {
            let launcher = launcher(app)?;
            let origin = launcher.origin();
            let _new = app.manage(launcher);
            window::open(app, origin)?;
            Ok(())
        })
        .build(tauri::generate_context!());
    match built {
        Ok(app) => app.run(|_app, event| on_event(&event)),
        Err(error) => tracing::error!(%error, "the app could not be built"),
    }
}

/// The app's lifecycle, logged (what Android does to the activity is
/// what the spike's notes ask about).
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "Tauri's RunEvent is non-exhaustive; only these are logged"
)]
fn on_event(event: &RunEvent) {
    match event {
        RunEvent::Ready => tracing::info!("ready"),
        RunEvent::Resumed => tracing::info!("resumed"),
        RunEvent::Exit => tracing::info!("exit"),
        _ => {}
    }
}

/// The book and the page's files in the app's private storage, and the
/// device's root certificates, for every launch.
///
/// # Errors
/// The app's private directories are unknown.
fn launcher(app: &App) -> tauri::Result<Launcher> {
    let paths = app.path();
    let dirs = AppDirs {
        config: paths.app_config_dir()?,
        data: paths.app_data_dir()?,
    };
    tracing::debug!(config = %dirs.config.display(), data = %dirs.data.display(), "app directories");
    // Read here, on the main thread before the page exists, not on the
    // runtime's workers: some hundred small files.
    let roots = Roots::system();
    if !roots.loaded() {
        tracing::warn!("no root certificates could be read; wss:// relays will fail");
    }
    Ok(Launcher::new(dirs, roots))
}

/// An error and its sources, for one log line.
fn error_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}
