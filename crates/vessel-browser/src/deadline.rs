//! How long a command may run. The broker's deadline is 15 seconds.
//! `browser_wait` with a `timeoutMs` gets that plus one second, and the
//! timeout is at most 30 seconds, so the deadline is at most 31.

use std::time::Duration;

use vessel_proto::browser::BrowserCommand;

/// The broker's deadline for every command but a wait that set `timeoutMs`.
pub const BROKER_DEADLINE: Duration = Duration::from_secs(15);

/// The deadline for `command`.
#[must_use]
pub fn deadline(command: &BrowserCommand) -> Duration {
    match command {
        BrowserCommand::Wait {
            timeout: Some(timeout),
            ..
        } => timeout.as_duration().saturating_add(Duration::from_secs(1)),
        BrowserCommand::Wait { timeout: None, .. }
        | BrowserCommand::ListTabs
        | BrowserCommand::NewTab { .. }
        | BrowserCommand::CloseTab { .. }
        | BrowserCommand::Navigate { .. }
        | BrowserCommand::Back { .. }
        | BrowserCommand::Forward { .. }
        | BrowserCommand::Reload { .. }
        | BrowserCommand::Snapshot { .. }
        | BrowserCommand::Screenshot { .. }
        | BrowserCommand::Click { .. }
        | BrowserCommand::Hover { .. }
        | BrowserCommand::Drag { .. }
        | BrowserCommand::Fill { .. }
        | BrowserCommand::Select { .. }
        | BrowserCommand::Type { .. }
        | BrowserCommand::Keypress { .. }
        | BrowserCommand::Upload { .. }
        | BrowserCommand::Scroll { .. }
        | BrowserCommand::Resize { .. }
        | BrowserCommand::Logs { .. }
        | BrowserCommand::Evaluate { .. } => BROKER_DEADLINE,
    }
}
