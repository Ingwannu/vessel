//! The app's updater (spikes/release/updater): started once at launch,
//! and inert unless the build compiled a feed's address and the release
//! key in (`VESSEL_UPDATE_FEED`, `VESSEL_UPDATE_KEY`; a development build
//! and every build until the owner names a public feed have neither).
//!
//! [Decision Log]
//! - Purpose: a build the owner installed finds the next one by itself
//!   and puts it in place, so each round of feedback needs no reinstall.
//! - Constraints: the updater runs on tokio; GPUI runs its own executor;
//!   the link's runtime is the link crate's own and stays private to it;
//!   this module is the release track's, merged into the desktop track's
//!   app, so it touches the app in as few places as it can.
//! - Alternatives: a task on the link's runtime (a change to the link
//!   crate's interface for a concern that is not the link's); a
//!   "restart to update" control in the shell (a change to its screens,
//!   left to the desktop track).
//! - Choice: a one-thread tokio runtime of its own, created only when the
//!   build can update; a GPUI task that follows the updater's state and,
//!   once a build is installed, says so in the frame's foot (the shell's
//!   notice). The installed build runs from the next start.
//! - Why: nothing is spent (no thread, no runtime, no request) in a build
//!   that cannot update; the app's other crates are not touched.
//! - Cost: one more thread while the app runs, when updates are on; no
//!   click to restart yet, so the owner restarts by quitting and opening
//!   the app (`Updater::restart` is there when the shell gets a control).
//!
//! `VESSEL_APP_NO_UPDATE` (any value) turns it off for one run.

use vessel_app_kit::ui::{App, AsyncApp, WeakEntity};
use vessel_app_updater::{Installation, UpdateState, Updater, settings_from_build};

use crate::shell::Shell;

/// Start the updater when this build can update; otherwise do nothing.
pub(crate) fn start(shell: WeakEntity<Shell>, cx: &mut App) {
    if std::env::var_os("VESSEL_APP_NO_UPDATE").is_some() {
        tracing::info!("updates are off for this run");
        return;
    }
    let settings = match settings_from_build() {
        Ok(Some(settings)) => settings,
        Ok(None) => return,
        Err(error) => {
            tracing::warn!(%error, "this build's update settings do not parse; no updates");
            return;
        }
    };
    let installation = match Installation::from_process(settings.target.default_packaging()) {
        Ok(installation) => installation,
        Err(error) => {
            tracing::warn!(%error, "the app could not find itself on disk; no updates");
            return;
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .thread_name("vessel-update")
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::warn!(%error, "the updater's runtime could not start; no updates");
            return;
        }
    };
    let updater = match Updater::start(settings, installation, runtime.handle()) {
        Ok(updater) => updater,
        Err(error) => {
            tracing::warn!(%error, "the updater could not start; no updates");
            return;
        }
    };
    cx.spawn(async move |cx| follow(updater, runtime, shell, cx).await)
        .detach();
}

/// Follow the updater's state for the app's life; say when a build is
/// installed. Owns the runtime, so the updater lives as long as this.
async fn follow(
    updater: Updater,
    runtime: tokio::runtime::Runtime,
    shell: WeakEntity<Shell>,
    cx: &mut AsyncApp,
) {
    let mut state = updater.state();
    loop {
        let now = state.borrow_and_update().clone();
        match now {
            UpdateState::Ready { label, .. } => {
                let said = shell.update(cx, |shell, cx| {
                    shell.notice = Some(shell.t.update_installed(&label.to_string()));
                    cx.notify();
                });
                if said.is_err() {
                    break;
                }
            }
            UpdateState::Failed { error } => {
                tracing::warn!(%error, "an update attempt failed; the next check tries again");
            }
            UpdateState::Idle
            | UpdateState::Checking
            | UpdateState::UpToDate { .. }
            | UpdateState::Downloading { .. } => {}
        }
        // The updater's task ends once a build is installed; its state
        // stays readable, and nothing more will change.
        if state.changed().await.is_err() {
            break;
        }
    }
    // Kept to here: dropping the runtime stops the updater's thread, which
    // must outlive every state it publishes.
    drop(updater);
    runtime.shutdown_background();
}
