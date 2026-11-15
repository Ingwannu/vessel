//! Typed interpretation of Codex replies; no session or provider process is started here.

use crate::error::{Disconnect, InputError};
use crate::input::Reply;
use serde_json::Value;
use vessel_proto::agents::AgentVersion;

/// A notification's parameters as `T`, or `None` (logged) when they do
/// not read.
pub(super) fn read<T: serde::de::DeserializeOwned>(method: &str, params: Value) -> Option<T> {
    match serde_json::from_value(params) {
        Ok(read) => Some(read),
        Err(error) => {
            tracing::warn!(method, %error, "a Codex notification that does not read");
            None
        }
    }
}

pub(super) fn handshake_failed(message: String) -> Disconnect {
    Disconnect::Handshake {
        message,
        refused: None,
        fork_refused: None,
    }
}

/// The version in Codex's user agent (`codex_cli_rs/0.166.0 (...)`).
pub(super) fn version_of(user_agent: &str) -> Option<AgentVersion> {
    let first = user_agent.split_whitespace().next()?;
    let version = first.split_once('/').map_or(first, |(_, version)| version);
    version.parse().ok()
}

pub(super) fn answer(reply: Reply, result: Result<(), InputError>) {
    if reply.send(result).is_err() {
        tracing::debug!("agent command reply dropped");
    }
}
