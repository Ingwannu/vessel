//! `terminal.create`'s profile (ADR 0117).

use serde::{Deserialize, Serialize};

use super::{Id, Prompt};

/// Run one of the node's terminal profiles instead of its shell: the node
/// finds `id` in its own list (saved, else shipped), adopts the prompt
/// placeholder, puts `prompt` (or nothing) in it and starts the program
/// with its arguments, without a shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(rename = "TerminalProfileLaunch")
)]
pub struct ProfileLaunch {
    /// Which profile.
    pub id: Id,
    /// The text for its placeholder: a new workspace's first message. Absent
    /// or empty, the arguments that are only the placeholder are left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<Prompt>,
}
