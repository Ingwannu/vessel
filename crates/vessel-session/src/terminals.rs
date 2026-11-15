//! Terminals beside sessions (protocol 11, ADR 0031): a shell in a PTY,
//! or a run of a project's script or service, each with a log of its own
//! that is carried exactly as a terminal session's is.
//!
//! Owns: one actor task per terminal that owns its process, numbers its
//! records (`EventSeq`, gap-free from zero), keeps them in a bounded ring
//! (the session runtime's [`Ring`](crate::ring), bounded here by bytes as
//! well as by count) and broadcasts them to followers; a service's
//! restarts. Must not: keep anything on disk (a terminal lives in the
//! node's memory only), decide who may open one, where, or which command a
//! script runs (the node decides, and hands over a `CommandSpec`), or know
//! the wire's calls.
//!
//! [Decision Log]
//! - Purpose: a shell beside a session is a terminal session without an
//!   agent; reusing the session's terminal path (the PTY, the ring, the
//!   records, the follow) keeps one transport for both and lets every
//!   client draw a shell as it draws a terminal session.
//! - Alternatives: a session actor with a flag (it would publish into the
//!   node's durable channel, so into the store, and carry a board, a
//!   conversation and an agent credential); a crate of its own (it would
//!   need a second ring and a second follow).
//! - Choice: a lean actor per terminal over the same ring, with no durable
//!   channel: nothing waits on a store, so a terminal never pauses for
//!   one, and its memory is bounded by the ring alone.
//! - Scripts and services run piped through `vessel_proc::spawn` (a
//!   process group on Unix, a kill-on-close job object on Windows), not in
//!   a PTY: `vessel-pty` cannot put a program in a job object before it
//!   starts (its Decision Log), and a service's grandchildren must end
//!   with it on every platform.

mod backoff;
mod cleanup;
mod handle;
mod log;
mod script;
mod shell;

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Duration;

pub use backoff::{Backoff, MAX_RESTARTS, Retry, STABLE};
pub use cleanup::{CleanupFact, CleanupObservation, CleanupReport};
pub use handle::{InputError, Page, RestartError, TerminalHandle};
pub use script::{ScriptStart, start_script};
pub use shell::{ShellStart, start_shell};

/// How much of a terminal's log is kept in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scrollback {
    /// Records kept, and the live followers' buffer.
    pub records: NonZeroUsize,
    /// Bytes of output kept.
    pub bytes: usize,
}

impl Scrollback {
    /// 4,096 records and 1 MiB: about 10,000 lines of 100 bytes (Paseo
    /// keeps 10,000 lines, GOAT 5,000 lines and 8 MiB); 64 live terminals
    /// hold at most 64 MiB.
    pub const DEFAULT: Self = Self {
        records: match NonZeroUsize::new(4096) {
            Some(records) => records,
            None => NonZeroUsize::MIN,
        },
        bytes: 1024 * 1024,
    };
}

/// Why a terminal could not start.
#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    /// The shell did not start in its PTY.
    #[error("the shell could not be started")]
    Spawn(#[source] vessel_pty::PtyError),
    /// The runtime is stopping; nothing starts any more.
    #[error("the node is stopping")]
    ShuttingDown,
    /// A blocking start did not come back.
    #[error("the start was interrupted")]
    Internal(#[source] tokio::task::JoinError),
}

/// How long a script or a service gets between `SIGTERM` and `SIGKILL`: a
/// dev server closes its connections and its watchers; longer than a
/// shell's two seconds (a shell is told the terminal closed, which it
/// acts on at once).
pub const SCRIPT_GRACE: Duration = Duration::from_secs(5);

/// A file to remove once a run has ended (the Windows command file a
/// script runs from, `vessel_workspace::commands`).
#[derive(Debug, Default)]
pub struct Leftover(pub Option<PathBuf>);
