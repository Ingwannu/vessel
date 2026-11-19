//! What "Node logs" shows below its path line (ADR 0111, TRACKER 569), as
//! Paseo's `DaemonLogsModal` with the coordinator's truthfulness rule.
//!
//! [Decision Log]
//! - Paseo opens its logs modal only once the logs have arrived, so it never
//!   shows a failure there. The app opens the dialog first, so it must say
//!   what happened: a failed call shows why, and "(log file is empty)" stands
//!   only for a file with no lines (the coordinator's ruling on row 334's
//!   last note; the page's `logContents` the same).
//! - A node at the protocol that answers `node.log` (33) has already dropped
//!   a terminal's escapes from its lines, so they are shown as they arrive.

use vessel_proto::diagnostics::NodeLog;

use super::Problem;

/// The contents below the path line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogContents {
    /// The lines, one per line.
    Lines(String),
    /// The file has no lines.
    Empty,
    /// The call failed.
    Error(Problem),
    /// The node keeps no log file; its path line says where it writes.
    None,
}

/// The contents for a `node.log` call's outcome.
#[must_use]
pub fn log_contents(log: &Result<NodeLog, Problem>) -> LogContents {
    match log {
        Err(problem) => LogContents::Error(*problem),
        Ok(NodeLog::NoFile { .. }) => LogContents::None,
        Ok(NodeLog::File { lines, .. }) if lines.is_empty() => LogContents::Empty,
        Ok(NodeLog::File { lines, .. }) => LogContents::Lines(lines.join("\n")),
    }
}
