//! `node.log` (ADR 0111, TRACKER 569): the last lines of the node's own
//! log, as Paseo's desktop shows its daemon's (`desktop_daemon_logs`, the
//! last 100 lines of `daemon.log`).
//!
//! [Decision Log]
//! - Purpose: a machine's administrator reads what its node wrote, from the
//!   page or the app, without a shell.
//! - Constraint: a log line can hold anything the node wrote (a path, an
//!   agent's error, words of a prompt). So this is its own call for `admin`,
//!   never part of the read-only, copied report (ADR 0074), and never
//!   forwarded: a node's administration is done there.
//! - Alternatives: Paseo's way, the desktop reading the file beside it (the
//!   page and the app reach machines over the network); a log tail inside
//!   `node.diagnostics` (Paseo's "Daemon log tail"; rejected above).
//! - Bounds: [`DEFAULT_LOG_LINES`] lines unless asked, at most
//!   [`MAX_LOG_LINES`], read from at most the last [`MAX_LOG_TAIL_BYTES`] of
//!   the file, each line cut at [`MAX_LOG_LINE_BYTES`]; the reply says when
//!   anything was cut.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Lines sent when the request names no count (Paseo's 100).
pub const DEFAULT_LOG_LINES: u32 = 100;
/// The most lines one reply holds.
pub const MAX_LOG_LINES: u32 = 1000;
/// The most bytes read from the end of the file.
pub const MAX_LOG_TAIL_BYTES: u64 = 64 * 1024;
/// The most bytes of one line sent.
pub const MAX_LOG_LINE_BYTES: usize = 4096;

/// `node.log`: the last `lines` lines of the node's log
/// ([`DEFAULT_LOG_LINES`] when `None`). Needs `admin`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ReadNodeLog {
    /// How many lines, at most [`MAX_LOG_LINES`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<u32>,
}

/// The node's log, as far as it has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum NodeLog {
    /// It writes to a file: its path and last lines.
    File {
        /// Where, the home folded to `~`.
        path: PathBuf,
        /// The last lines, oldest first, each at most
        /// [`MAX_LOG_LINE_BYTES`].
        lines: Vec<String>,
        /// Whether earlier lines exist that were not read (past the count
        /// or the byte bound), or a line was cut.
        cut: bool,
    },
    /// It writes no file this node can name.
    NoFile {
        /// Where it writes instead.
        writes_to: LogSink,
    },
}

/// Where a node without a log file writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum LogSink {
    /// A terminal: it was started in the foreground.
    Terminal,
    /// Something else (a service manager's journal, a pipe); not readable
    /// here.
    Elsewhere,
}
