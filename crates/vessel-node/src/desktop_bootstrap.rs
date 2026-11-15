//! A one-shot managed pipe keeps its original instance through proof and RPC.
//!
//! Canonical IPC never calls this pre-auth path. A random name is not authority:
//! the launcher correlates a fresh challenge with inherited child stdout before
//! presenting the token on this same retained server instance.

/// Typed failures never include the private pipe name or transcript payload.
#[derive(Debug, thiserror::Error)]
pub enum DesktopBootstrapError {
    /// This platform cannot establish the approved private serving boundary.
    #[error("private desktop bootstrap is unsupported")]
    Unsupported,
    /// Only fixed bounded transcript records are accepted.
    #[error(transparent)]
    Transcript(#[from] vessel_proto::local_launch::TranscriptError),
    /// Pipe creation, opening or fixed-record I/O failed.
    #[error("private desktop bootstrap I/O failed")]
    Io(#[from] std::io::Error),
    /// Invalid UTF-8 is diagnosed without retaining the record bytes.
    #[error("private desktop bootstrap text is invalid")]
    Encoding(#[from] std::str::Utf8Error),
    /// A fresh client must prove its one connection within a finite bound.
    #[error("private desktop bootstrap timed out")]
    Deadline(#[from] tokio::time::error::Elapsed),
    /// Node teardown closes the pending private instance.
    #[error("private desktop bootstrap stopped")]
    Stopped,
    /// The original tracked task closed before it delivered a proof result.
    #[error("private desktop bootstrap task closed")]
    Worker(#[from] tokio::sync::oneshot::error::RecvError),
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(crate) use windows::{Proof, bind, start};
