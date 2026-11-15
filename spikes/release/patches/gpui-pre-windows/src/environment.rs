// MODIFIED by Vessel (Team-WICKED), 2026-11-15: a file of Vessel's, not in
// upstream's crate (spikes/release/patches/README.md, "gpui-pre-windows",
// change 6); the module line in `gpui_windows.rs` is marked "Vessel patch".

//! Telling running programs that the user's environment changed.

use std::time::Duration;

use windows::Win32::Foundation::{ERROR_SUCCESS, ERROR_TIMEOUT, LPARAM, SetLastError, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
};
use windows::core::w;

/// How long the broadcast may take before it gives up on a window that
/// does not answer.
pub const ENVIRONMENT_BROADCAST_TIMEOUT: Duration = Duration::from_secs(5);

/// Why the broadcast did not finish. In every case the environment itself
/// is already written; what is unknown is which running programs were told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentBroadcastError {
    /// A window did not answer within [`ENVIRONMENT_BROADCAST_TIMEOUT`].
    /// Windows that answered have the new environment.
    TimedOut,
    /// The call reported failure without an error code. A broadcast to
    /// every window can end this way (the result is not per window), so
    /// it is neither a timeout nor a refusal: whether the windows were
    /// told is not known.
    Unconfirmed,
    /// The system refused the broadcast, with this error.
    Failed(windows::core::Error),
}

/// Tells every top-level window that the user's environment changed
/// (`WM_SETTINGCHANGE` with `"Environment"`), as Windows' own settings do
/// after the user edits a variable: Explorer then gives programs it starts
/// the new value (a new terminal finds a directory just added to `PATH`).
/// A hung window is skipped (`SMTO_ABORTIFHUNG`); the call returns within
/// [`ENVIRONMENT_BROADCAST_TIMEOUT`] per window that does not answer.
///
/// Call it after writing the variable (`HKEY_CURRENT_USER\Environment`),
/// off the UI thread: it waits for the windows' answers.
pub fn broadcast_environment_changed() -> Result<(), EnvironmentBroadcastError> {
    let timeout = u32::try_from(ENVIRONMENT_BROADCAST_TIMEOUT.as_millis()).unwrap_or(u32::MAX);
    let section = w!("Environment");
    let mut answer: usize = 0;
    // SAFETY: `section` is a static, NUL-terminated wide string that
    // outlives the call (receivers read it during their answer, which the
    // call waits for), and `answer` is a live `usize` for the result.
    // SetLastError takes a plain value. The thread's last error is cleared
    // first so that a failure that sets no code reads as no code, not as
    // whatever an earlier call on this thread left behind.
    let sent = unsafe {
        SetLastError(ERROR_SUCCESS);
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(section.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            timeout,
            Some(&mut answer),
        )
    };
    if sent.0 != 0 {
        return Ok(());
    }
    // Read at once: anything called after this could replace it.
    let error = windows::core::Error::from_thread();
    let code = error.code();
    if code == windows::core::HRESULT::from_win32(ERROR_TIMEOUT.0) {
        Err(EnvironmentBroadcastError::TimedOut)
    } else if code.is_ok() {
        // No code: an `Error` here would read "The operation completed
        // successfully", which says the opposite of what happened.
        Err(EnvironmentBroadcastError::Unconfirmed)
    } else {
        Err(EnvironmentBroadcastError::Failed(error))
    }
}
