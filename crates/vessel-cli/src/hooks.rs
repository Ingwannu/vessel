//! `vessel hooks <agent> <event>`: an agent's hook reports the activity of
//! the Vessel terminal it runs in (ADR 0089, "The CLI consumer: agent
//! hooks"; Paseo 0.15.4 `cli/src/commands/hooks.ts`).
//!
//! [Decision Log]
//! - Purpose: Claude Code, Codex and `OpenCode` run their hooks for every
//!   turn. The node's "Terminal agent hooks" setting installs this command
//!   there (`vessel_node::agent_hooks`), so the terminal's attention follows
//!   the agent: working, finished, needs input.
//! - Choice, as Paseo's:
//!   - outside a Vessel terminal (no `VESSEL_TERMINAL_ACTIVITY_*`), and for
//!     an agent or event without a mapping, it does nothing;
//!   - the input is read only for Claude Code's `Notification`, waiting at
//!     most 100 ms for each piece (Paseo's `hooks.ts:63-76`, the 100 ms at `:68`);
//!   - the report is given 500 ms (`hooks.ts:98`);
//!   - every failure is silent: a hook must never disturb the agent, and the
//!     command always exits 0.
//! - Vessel's transport: the terminal's activity-only capability on the
//!   node's local endpoint (`LocalCredential::TerminalActivity`), where
//!   Paseo posts to a URL with a token. It sends `terminal.activity` with no
//!   revision, which only a capability may do; the node scopes it to that
//!   terminal.
//! - While ADR 0089's protocol is unassigned, the node gives terminals no
//!   capability and the client refuses the call, so this reports nothing.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use tokio::io::AsyncReadExt as _;
use vessel_client::{LocalCredential, NodeClient};
use vessel_node::agent_hooks::Event;
use vessel_proto::ClientInfo;
use vessel_proto::notifications::{
    ReportTerminalActivity, TERMINAL_ACTIVITY_ID_ENV, TERMINAL_ACTIVITY_SECRET_ENV,
    TERMINAL_ACTIVITY_SOCKET_ENV, TerminalActivityAuth, TerminalActivitySignal,
};

/// How long the input may pause before it is given up (Paseo's 100 ms).
const INPUT_IDLE: Duration = Duration::from_millis(100);

/// The most input read; a notification is far smaller.
const INPUT_MOST: usize = 1024 * 1024;

/// How long the report may take, connection included (Paseo's 500 ms).
const REPORT: Duration = Duration::from_millis(500);

/// Report `event` from `agent`'s hook; silent whatever happens.
pub(crate) async fn run(agent: &str, event: &str) {
    let Some(target) = target(|name| std::env::var_os(name)) else {
        return;
    };
    let Some(event) = Event::parse(agent, event) else {
        return;
    };
    let input = if event.reads_input() && !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        read_input().await
    } else {
        None
    };
    let Some(signal) = event.signal(input.as_deref()) else {
        return;
    };
    if tokio::time::timeout(REPORT, report(target, signal))
        .await
        .is_err()
    {
        tracing::debug!("a hook's activity report took too long and was dropped");
    }
}

/// The terminal this runs in, and its node's local endpoint.
#[derive(Debug)]
struct Target {
    auth: TerminalActivityAuth,
    socket: PathBuf,
}

/// The capability the node put in this terminal's environment; `None`
/// outside a Vessel terminal or with any part missing or unreadable.
fn target(read: impl Fn(&str) -> Option<OsString>) -> Option<Target> {
    let terminal = read(TERMINAL_ACTIVITY_ID_ENV)?.to_str()?.parse().ok()?;
    let secret = read(TERMINAL_ACTIVITY_SECRET_ENV)?.to_str()?.parse().ok()?;
    let socket = PathBuf::from(read(TERMINAL_ACTIVITY_SOCKET_ENV)?);
    if socket.as_os_str().is_empty() {
        return None;
    }
    Some(Target {
        auth: TerminalActivityAuth { terminal, secret },
        socket,
    })
}

/// The hook's input, or `None` when it pauses for longer than
/// [`INPUT_IDLE`], fails, or is longer than [`INPUT_MOST`] (Paseo's
/// `readInput`: a pause gives up the whole input).
async fn read_input() -> Option<String> {
    let mut stdin = tokio::io::stdin();
    let mut text = Vec::new();
    let mut piece = [0_u8; 8192];
    loop {
        let read = tokio::time::timeout(INPUT_IDLE, stdin.read(&mut piece))
            .await
            .ok()?
            .ok()?;
        if read == 0 {
            return String::from_utf8(text).ok();
        }
        text.extend_from_slice(piece.get(..read)?);
        if text.len() > INPUT_MOST {
            return None;
        }
    }
}

/// Connect with the terminal's capability and report `signal`.
async fn report(target: Target, signal: TerminalActivitySignal) {
    let terminal = target.auth.terminal;
    let client = NodeClient::connect_local(
        &target.socket,
        &LocalCredential::TerminalActivity(target.auth),
        ClientInfo {
            name: "vessel-hooks".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        },
    )
    .await;
    let Ok(client) = client else {
        return;
    };
    let reported = client
        .report_terminal_activity(ReportTerminalActivity {
            terminal,
            revision: None,
            signal,
        })
        .await;
    if reported.is_err() {
        tracing::debug!("a hook's activity report was not accepted");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let pairs: Vec<(String, OsString)> = pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        move |name| {
            pairs
                .iter()
                .find(|(known, _value)| known == name)
                .map(|(_name, value)| value.clone())
        }
    }

    #[test]
    fn only_a_vessel_terminals_whole_capability_is_a_target() {
        let terminal = vessel_proto::TerminalId::random();
        let secret = vessel_proto::notifications::TerminalActivitySecret::from_bytes([3; 32]);
        let id = terminal.to_string();
        let key = secret.to_string();
        let whole = [
            (TERMINAL_ACTIVITY_ID_ENV, id.as_str()),
            (TERMINAL_ACTIVITY_SECRET_ENV, key.as_str()),
            (TERMINAL_ACTIVITY_SOCKET_ENV, "/run/vessel/node.sock"),
        ];
        let found = target(env(&whole)).unwrap();
        assert_eq!(found.auth.terminal, terminal);
        assert_eq!(found.auth.secret, secret);
        assert_eq!(found.socket, PathBuf::from("/run/vessel/node.sock"));
        for missing in 0..whole.len() {
            let mut part = whole.to_vec();
            let _gone = part.remove(missing);
            assert!(target(env(&part)).is_none(), "without {}", whole[missing].0);
        }
        let mut wrong = whole.to_vec();
        wrong[0].1 = "not-a-terminal";
        assert!(target(env(&wrong)).is_none());
        let mut empty = whole.to_vec();
        empty[2].1 = "";
        assert!(target(env(&empty)).is_none());
        assert!(
            !format!("{found:?}").contains(&key),
            "the secret is never printed"
        );
    }
}
