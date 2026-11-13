//! Starting the updated build, so the running one can quit.
//!
//! Zed restarts through its platform layer (on Windows, a PowerShell
//! script that waits for the old process); Tauri's `restart` spawns the
//! executable again and exits. Vessel spawns the new executable with the
//! same arguments and returns: the app then quits through its own UI
//! layer (GPUI's `quit`), which closes windows and lets the runtime stop
//! in order, rather than exiting from inside a library as
//! cargo-packager's updater does on Windows.

use std::{ffi::OsString, process::Stdio};

use crate::{install::Installation, label::Label};

/// Set in the new process's environment to the build that started it, so
/// the app can say it was updated.
pub const RESTARTED_FROM: &str = "VESSEL_APP_RESTARTED_FROM";

/// Why the new build did not start.
#[derive(Debug, thiserror::Error)]
#[error("the updated app could not be started")]
pub struct RestartError(#[source] std::io::Error);

/// Start `installation`'s executable with `args` (the running process's
/// arguments after the first), marked as restarted from `running`.
///
/// # Errors
/// [`RestartError`] when the process does not start; the running app
/// should then stay open.
pub fn relaunch(
    installation: &Installation,
    args: impl IntoIterator<Item = OsString>,
    running: &Label,
) -> Result<(), RestartError> {
    let child = std::process::Command::new(installation.executable())
        .args(args)
        .env(RESTARTED_FROM, running.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(RestartError)?;
    // Not waited for: it is the app from now on and outlives this
    // process, which quits next (its parent becomes the system's).
    drop(child);
    Ok(())
}
