//! [`read_login_status`]: what a harness's own status command
//! ([`crate::login_check`]) said, read as a typed answer for
//! `agent.status` (protocol 11, ADR 0032).
//!
//! [Decision Log]
//! - Purpose: a person sees whether a profile is signed in before a session
//!   fails on it, without a turn (a turn costs the owner money).
//! - Constraint: the commands' output is the vendors', and changes with
//!   their versions; what they print may hold an account's e-mail address,
//!   so it is never logged, stored or sent: only the answer leaves here.
//! - What each says (from their `--help` and agent-orchestrator's reading
//!   of the same commands): Claude Code's `claude auth status` prints JSON
//!   by default, with a boolean `loggedIn`; Codex's `codex login status`
//!   prints a line, "Logged in using ..." or "Not logged in".
//! - Choice: Claude Code by its JSON field only; Codex by those words,
//!   ignoring case, "not logged in" checked first. Anything else is
//!   [`LoginStatus::Unread`] (another version's words), never a guess
//!   from the exit code, which `login_check` notes was never tried
//!   signed out.
//! - Holding credentials is not the same as their being valid: a turn
//!   tells that. The answer says only what the command said.

use vessel_proto::HarnessKind;

/// What a status command said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginStatus {
    /// It holds credentials.
    Yes,
    /// It does not.
    No,
    /// Its words are not read here.
    Unread,
}

/// The longest output read from a status command; past it, the answer is
/// [`LoginStatus::Unread`].
pub const MAX_STATUS_OUTPUT: usize = 64 * 1024;

/// What `harness`'s status command said on `stdout` and `stderr`.
#[must_use]
pub fn read_login_status(harness: HarnessKind, stdout: &[u8], stderr: &[u8]) -> LoginStatus {
    if stdout.len() > MAX_STATUS_OUTPUT || stderr.len() > MAX_STATUS_OUTPUT {
        return LoginStatus::Unread;
    }
    match harness {
        HarnessKind::ClaudeCode => claude(stdout),
        HarnessKind::Codex => codex(stdout, stderr),
        // No status command (`login_check` answers `None`).
        HarnessKind::Gemini | HarnessKind::OpenCode => LoginStatus::Unread,
    }
}

/// `{"loggedIn": true, ...}`.
fn claude(stdout: &[u8]) -> LoginStatus {
    let Ok(serde_json::Value::Object(status)) = serde_json::from_slice(stdout) else {
        return LoginStatus::Unread;
    };
    match status.get("loggedIn") {
        Some(serde_json::Value::Bool(true)) => LoginStatus::Yes,
        Some(serde_json::Value::Bool(false)) => LoginStatus::No,
        Some(_) | None => LoginStatus::Unread,
    }
}

/// "Logged in using ..." or "Not logged in", on either stream.
fn codex(stdout: &[u8], stderr: &[u8]) -> LoginStatus {
    let text = [stdout, stderr]
        .iter()
        .map(|bytes| String::from_utf8_lossy(bytes).to_lowercase())
        .collect::<Vec<_>>()
        .join("\n");
    if text.contains("not logged in") {
        LoginStatus::No
    } else if text
        .lines()
        .any(|line| line.trim_start().starts_with("logged in"))
    {
        LoginStatus::Yes
    } else {
        LoginStatus::Unread
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_status_command_is_read_by_its_own_words() {
        let read = |harness, out: &str, err: &str| {
            read_login_status(harness, out.as_bytes(), err.as_bytes())
        };
        let claude = HarnessKind::ClaudeCode;
        assert_eq!(
            read(claude, r#"{"loggedIn":true,"authMethod":"x"}"#, ""),
            LoginStatus::Yes
        );
        assert_eq!(read(claude, r#"{"loggedIn":false}"#, ""), LoginStatus::No);
        for unread in [r#"{"loggedIn":"yes"}"#, "Logged in", "", "[]"] {
            assert_eq!(read(claude, unread, ""), LoginStatus::Unread, "{unread}");
        }
        let codex = HarnessKind::Codex;
        assert_eq!(
            read(codex, "", "Logged in using ChatGPT\n"),
            LoginStatus::Yes
        );
        assert_eq!(read(codex, "Not logged in\n", ""), LoginStatus::No);
        assert_eq!(read(codex, "Signed in?\n", ""), LoginStatus::Unread);
        assert_eq!(
            read(HarnessKind::Gemini, "Logged in", ""),
            LoginStatus::Unread
        );
        let long = "x".repeat(MAX_STATUS_OUTPUT + 1);
        assert_eq!(read(codex, &long, "Logged in"), LoginStatus::Unread);
    }
}
