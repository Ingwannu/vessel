//! Browser builds retain their existing in-memory preference path.

/// A missing native client cannot authorize replacing a retained terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerminalPreferenceState {
    Unmanaged,
    Applied(vessel_app_view::terminal::ScrollbackLines),
    Unavailable,
}

pub(crate) fn prepare_terminal_opening(
    shell: &mut crate::shell::Shell,
    cx: &mut gpui::Context<'_, crate::shell::Shell>,
) -> TerminalPreferenceState {
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::desktop::preferences::drain_terminal(shell, cx)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _browser = (shell, cx);
        TerminalPreferenceState::Unmanaged
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use crate::desktop::preferences::{refused, rollback, start, submit};

#[cfg(target_arch = "wasm32")]
pub(crate) fn start(
    _shell: &mut crate::shell::Shell,
    _window: &mut gpui::Window,
    _cx: &mut gpui::Context<'_, crate::shell::Shell>,
) {
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn submit(
    _shell: &crate::shell::Shell,
    command: vessel_app_link::Command,
) -> Result<Option<vessel_app_link::Command>, vessel_app_link::Unsent> {
    Ok(Some(command))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn refused(_shell: &mut crate::shell::Shell, _unsent: &vessel_app_link::Unsent) {}
#[cfg(target_arch = "wasm32")]
pub(crate) fn rollback(
    _shell: &mut crate::shell::Shell,
    _window: &mut gpui::Window,
    _cx: &mut gpui::Context<'_, crate::shell::Shell>,
) {
}
