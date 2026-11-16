//! Irreversible history trimming follows only the same writer's Saved delivery.

use crate::areas::desktop::preferences::TerminalPreferenceState;
use crate::desktop::Desktop;
use crate::shell::Shell;
use gpui::Context;

pub(crate) fn drain_terminal(
    shell: &mut Shell,
    cx: &mut Context<'_, Shell>,
) -> TerminalPreferenceState {
    let Some(client) = &shell.areas.desktop.preferences else {
        return if cx.has_global::<Desktop>() {
            TerminalPreferenceState::Unavailable
        } else {
            TerminalPreferenceState::Unmanaged
        };
    };
    let batch = match client.drain_scrollback() {
        Ok(batch) => batch,
        Err(error) => {
            tracing::warn!(%error, "committed terminal preference delivery is unavailable");
            shell.notice = Some(shell.words.desktop_shell().preferences_failed().to_owned());
            cx.notify();
            return TerminalPreferenceState::Unavailable;
        }
    };
    // This Cell belongs to the UI Client, not a fresh global read. A later save
    // remains pending for the successor after this exact drain boundary.
    let applied = client.applied_scrollback();
    if let Some(batch) = batch {
        shell.terminal_scrollback_changed(batch.minimum(), cx);
        if batch.latest().limit() != batch.minimum() {
            shell.terminal_scrollback_changed(batch.latest().limit(), cx);
        }
    }
    TerminalPreferenceState::Applied(applied)
}
