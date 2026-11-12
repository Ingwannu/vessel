//! Which TCP ports the process trees Vessel started listen on, found
//! without a privileged call (protocol 11, ADR 0031).
//!
//! [Decision Log]
//! - Purpose: a session's agent or a project's service starts a dev server
//!   and a person wants to know its port; the node reports the listeners
//!   of the trees it started (its sessions' and its terminals').
//! - What each platform allows without privilege:
//!   - Linux: `/proc` holds every process's parent (`stat`), its
//!     descriptors (`fd`, readable for the node's own user's processes,
//!     which is every process Vessel started) and the kernel's socket
//!     table (`net/tcp`, `net/tcp6`), so nothing is run.
//!   - macOS: no `/proc`; `ps` lists parents and `lsof` lists the
//!     listening sockets of the user's own processes (GOAT does the same).
//!   - Windows: `netstat -ano` lists every listener with its process id,
//!     and `PowerShell`'s `Win32_Process` query gives parents and image
//!     names; both need no administrator. Where `PowerShell` may not run,
//!     the scan fails and says so.
//! - Alternatives: a crate (`sysinfo`, `netstat2`): a dependency, and on
//!   macOS and Windows they call the same system interfaces through
//!   `unsafe` FFI this workspace forbids in its own code, for a scan run at
//!   most every 3 s; parsing the tools' text is what GOAT does.
//! - Bounds (`ScanLimits`): the processes walked in the trees, the
//!   descriptors read, and each tool's time and output. A scan past a
//!   bound returns what it found and says it stopped (`truncated`).
//! - A listener belongs to the tree that holds its process at the scan; a
//!   process that left its tree (a daemon that detached and was adopted by
//!   `init`) is not reported, and is not Vessel's to stop either.

mod parse;
mod tree;

use std::net::IpAddr;
use std::time::Duration;

pub use tree::Trees;

/// A listening socket found in one of the trees.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Listener {
    /// The root of the tree it was found in (one of those asked for).
    pub root: u32,
    /// The listening process.
    pub pid: u32,
    /// Its name, where the system said.
    pub process: Option<String>,
    /// The local address.
    pub address: IpAddr,
    /// The port.
    pub port: u16,
}

/// What one scan may cost.
#[derive(Debug, Clone, Copy)]
pub struct ScanLimits {
    /// Processes walked in the trees, all trees together.
    pub processes: usize,
    /// Descriptors read (Linux).
    pub descriptors: usize,
    /// How long one tool may run (macOS, Windows).
    pub tool_time: Duration,
    /// How much of one tool's output is read.
    pub tool_output: usize,
}

impl ScanLimits {
    /// The node's: 4,096 processes, 50,000 descriptors (GOAT's), 5 s and
    /// 4 MiB a tool.
    pub const DEFAULT: Self = Self {
        processes: 4096,
        descriptors: 50_000,
        tool_time: Duration::from_secs(5),
        tool_output: 4 * 1024 * 1024,
    };
}

/// What a scan found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scan {
    /// The listeners, at most one per process, address and port.
    pub listeners: Vec<Listener>,
    /// Whether a bound stopped the scan before it looked everywhere.
    pub truncated: bool,
}

/// Why a scan found nothing it could report.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    /// A source could not be read (`/proc`, or a tool could not start).
    #[error("cannot read {source_name}")]
    Read {
        /// What.
        source_name: &'static str,
        /// The system's error.
        #[source]
        error: std::io::Error,
    },
    /// A tool ran past its time.
    #[error("{tool} did not answer within its time")]
    TimedOut {
        /// Which.
        tool: &'static str,
    },
    /// A tool failed.
    #[error("{tool} failed (exit code {code:?})")]
    Failed {
        /// Which.
        tool: &'static str,
        /// Its exit code.
        code: Option<i32>,
    },
}

/// The listeners of the trees rooted at `roots`. Each process is reported
/// under the first root whose tree holds it.
///
/// # Errors
/// [`ScanError`] when a source cannot be read at all.
pub async fn scan(roots: &[u32], limits: ScanLimits) -> Result<Scan, ScanError> {
    if roots.is_empty() {
        return Ok(Scan::default());
    }
    os::scan(roots, limits).await
}

#[cfg(target_os = "linux")]
#[path = "listening/linux.rs"]
mod os;

#[cfg(any(target_os = "macos", windows))]
#[path = "listening/tools.rs"]
mod os;

/// Elsewhere (the BSDs): nothing is scanned, and the node says so.
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod os {
    use super::{Scan, ScanError, ScanLimits};

    pub(super) async fn scan(_roots: &[u32], _limits: ScanLimits) -> Result<Scan, ScanError> {
        Err(ScanError::Read {
            source_name: "this system's sockets",
            error: std::io::Error::from(std::io::ErrorKind::Unsupported),
        })
    }
}
