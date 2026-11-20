//! [`Launcher`]: a fresh launch of the bridge each time the web view
//! shows the app's start page, and the page opened at it.
//!
//! [Decision Log]
//! - Purpose: the page redeems a launch's bootstrap once (ADR 0017); a
//!   web view that loads the launch address a second time is refused for
//!   good ("The UI was restarted or its identity could not be verified").
//! - Seen in the emulator: on a cold start the web view loaded the launch
//!   address more than once and the page was refused; and Tauri, when
//!   Android makes the activity again in a living process, reloads the
//!   window's first address. So a launch address given to the web view
//!   once, at start, is not enough.
//! - Alternatives: making the bootstrap reusable by this app (weakens the
//!   bridge's one rule that keeps other apps on the phone out); watching
//!   page loads for a second load of the launch address (the address of a
//!   load is not reliably reported with its fragment, and a refused page
//!   loads normally).
//! - Choice: the window's own address is the app's start page (in the
//!   APK, `dist/index.html`); whenever it finishes loading, the launcher
//!   stops the bridge it ran (if any), starts a new one with a new launch,
//!   and sends the web view there. A web view made again starts at the
//!   start page and so gets a launch of its own.
//! - Cost: a new launch closes the old page's socket; the old page is gone
//!   by then (its web view reloaded). Drafts and preferences are in the
//!   bridge's files, so nothing typed is lost; a session's stream resumes
//!   from the node.

use std::sync::{Arc, Mutex};

use tauri::async_runtime::Mutex as AsyncMutex;
use tauri::{AppHandle, Manager as _, Url, WebviewWindow};
use vessel_phone_core::{AppDirs, Bridge, PhoneSource, Roots};

use crate::window::Origin;

/// The name a node lists this device under once paired.
const DEVICE_NAME: &str = "Vessel app (phone)";

/// The start page's address when the bridge could not start (the page
/// shows the failure for this fragment).
const FAILED: &str = "failed";

/// What a launch needs, and the launch running now.
#[derive(Debug)]
pub(crate) struct Launcher {
    dirs: AppDirs,
    roots: Roots,
    /// The bridge serving the page now. An async lock: held across the
    /// old bridge's stop and the new one's start, so launches never
    /// overlap (a load while one is under way is ignored).
    current: AsyncMutex<Option<Bridge>>,
    /// The origin the web view may show now, read by the navigation guard
    /// on the web view's thread (a short `std` lock, never across await).
    origin: Arc<Mutex<Option<Origin>>>,
}

impl Launcher {
    /// A launcher keeping the app's files under `dirs` and checking
    /// relays against `roots`.
    pub(crate) fn new(dirs: AppDirs, roots: Roots) -> Self {
        Self {
            dirs,
            roots,
            current: AsyncMutex::new(None),
            origin: Arc::new(Mutex::new(None)),
        }
    }

    /// The bridge's origin as the navigation guard sees it.
    pub(crate) fn origin(&self) -> Arc<Mutex<Option<Origin>>> {
        Arc::clone(&self.origin)
    }
}

/// The start page finished loading at `url` in `webview`: launch, unless
/// it is the failure page or a launch is under way.
pub(crate) fn start_page_loaded(webview: &WebviewWindow, url: &Url) {
    if url.fragment() == Some(FAILED) {
        return;
    }
    let webview = webview.clone();
    drop(tauri::async_runtime::spawn(async move {
        let app = webview.app_handle().clone();
        launch(&app, &webview).await;
    }));
}

/// Stop the old bridge, start a new one, send the web view to it.
async fn launch(app: &AppHandle, webview: &WebviewWindow) {
    let launcher = app.state::<Launcher>();
    let Ok(mut current) = launcher.current.try_lock() else {
        tracing::debug!("a launch is under way; this load is ignored");
        return;
    };
    if let Some(old) = current.take()
        && let Err(error) = old.stop().await
    {
        tracing::warn!(error = %crate::error_chain(&error), "the previous bridge stopped badly");
    }
    let source = PhoneSource::new(
        launcher.dirs.clone(),
        launcher.roots.clone(),
        DEVICE_NAME.to_owned(),
    );
    let started = Bridge::start(source).await.map_err(LaunchError::Start).and_then(|bridge| {
        let url: Url = bridge
            .url()
            .expose()
            .parse()
            .map_err(LaunchError::Address)?;
        let origin = Origin::of(&url).ok_or(LaunchError::NotHttp)?;
        Ok((bridge, url, origin))
    });
    match started {
        Ok((bridge, url, origin)) => {
            // The guard learns the new origin before the web view goes there.
            if let Ok(mut allowed) = launcher.origin.lock() {
                *allowed = Some(origin);
            }
            if let Err(error) = webview.navigate(url) {
                tracing::error!(%error, "the web view did not take the launch address");
            }
            *current = Some(bridge);
        }
        Err(error) => {
            tracing::error!(error = %crate::error_chain(&error), "the bridge did not start");
            failed(webview);
        }
    }
}

/// Show the start page's failure text.
fn failed(webview: &WebviewWindow) {
    let Ok(mut url) = webview.url() else {
        return;
    };
    url.set_fragment(Some(FAILED));
    if let Err(error) = webview.navigate(url) {
        tracing::error!(%error, "the failure page could not be shown");
    }
}

/// Why a launch failed.
#[derive(Debug, thiserror::Error)]
enum LaunchError {
    #[error(transparent)]
    Start(vessel_phone_core::StartError),
    #[error("the launch address does not parse")]
    Address(#[source] url::ParseError),
    #[error("the launch address is not the bridge's kind")]
    NotHttp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_launcher_starts_with_no_origin_allowed() {
        let launcher = Launcher::new(
            AppDirs {
                config: "/nowhere/files".into(),
                data: "/nowhere/data".into(),
            },
            Roots::none(),
        );
        assert!(launcher.origin().lock().unwrap().is_none());
    }
}
