//! How long a run may take and how much it may print: a local command's
//! limits, a network one's and a clone's. Enforcing them is `output`'s.

use std::time::Duration;

use super::{KIB, MIB};

/// How long a run may take and how much it may print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Limits {
    /// Killed after this.
    pub(crate) time: Duration,
    /// Standard output kept; the rest is cut.
    pub(crate) stdout: usize,
    /// Standard error kept; the rest is cut.
    pub(crate) stderr: usize,
}

impl Limits {
    /// A local command: status, diff, log, commit, a worktree.
    pub(crate) const LOCAL: Self = Self {
        time: Duration::from_secs(30),
        stdout: 16 * MIB,
        stderr: 64 * KIB,
    };

    /// A command that talks to a remote: push, pull, fetch.
    pub(crate) const NETWORK: Self = Self {
        time: Duration::from_secs(120),
        stdout: MIB,
        stderr: 64 * KIB,
    };

    /// A clone, whose progress streams on standard error.
    pub(crate) const CLONE: Self = Self {
        time: Duration::from_secs(3600),
        stdout: 64 * KIB,
        // Progress is handed on as it comes (`output_streaming`); only the
        // tail is kept, as for every run.
        stderr: 64 * KIB,
    };

    /// These limits with standard output capped at `bytes`.
    pub(crate) const fn with_stdout(self, bytes: usize) -> Self {
        Self {
            stdout: bytes,
            ..self
        }
    }
}
