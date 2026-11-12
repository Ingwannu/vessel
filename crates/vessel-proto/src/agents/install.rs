//! `agent.install` and `agent.install_output` (protocol 11, ADR 0032):
//! running one of the four's documented installers on the node, and
//! reading what it prints.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::added::InstallPart;
use crate::harness::HarnessKind;
use crate::id::InstallId;
use crate::terminal::TerminalBytes;

/// `agent.install`: run the vendor's documented command for `agent`'s
/// `part` (the one `agent.list` shows). Needs `admin`; refused to an
/// agent and to a call from another node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InstallAgent {
    /// Which of the four.
    pub agent: HarnessKind,
    /// It, or its ACP adapter.
    pub part: InstallPart,
}

/// An install the node started.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InstallJob {
    /// Its id, for `agent.install_output`.
    pub id: InstallId,
    /// Which of the four.
    pub agent: HarnessKind,
    /// It, or its adapter.
    pub part: InstallPart,
    /// The command, as a person would type it.
    pub command: String,
    /// When it started.
    pub started_at: DateTime<Utc>,
}

/// `agent.install_output`: a page of an install's output. Needs `admin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadInstall {
    /// Which install.
    pub job: InstallId,
    /// From this byte of its output.
    #[serde(default)]
    pub from: u64,
    /// Wait this many seconds (at most 30) for more output or its end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait: Option<u32>,
}

/// A page of an install's output, the shape of `worktree.output`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InstallOutput {
    /// Where this page starts.
    pub from: u64,
    /// Standard output and error together, as printed.
    pub bytes: TerminalBytes,
    /// Where the next page starts.
    pub next: u64,
    /// Whether the output reached its limit and the rest was not kept.
    pub truncated: bool,
    /// The install now.
    pub state: InstallState,
}

/// An install's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum InstallState {
    /// Running.
    Running,
    /// It exited with 0.
    Succeeded,
    /// It exited otherwise (`code` absent: stopped by a signal), or did
    /// not start.
    Failed {
        /// Its exit code.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<i32>,
    },
    /// It ran past its 15 minutes and was stopped, with its children.
    TimedOut,
}
