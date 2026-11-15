//! Closed fixture failures; raw framed peer diagnostics never leave the owner.
use std::{fmt, future::Future, io};

/// The named bounded wait that failed in an isolated regression scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Construct the isolated runtime/peer.
    Setup,
    /// Read one bounded frame.
    Read,
    /// Write one bounded frame.
    Write,
    /// Apply a real actor command.
    Dispatch,
    /// Await a scenario control response.
    Control,
    /// Cancel and join the owned peer.
    Close,
    /// Create or remove the isolated book directory.
    Home,
}
impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Setup => "setup",
            Self::Read => "frame read",
            Self::Write => "frame write",
            Self::Dispatch => "dispatch",
            Self::Control => "control",
            Self::Close => "close",
            Self::Home => "isolated directory",
        })
    }
}

/// Test failures retain bounded categories, never a frame, credential or path.
#[derive(thiserror::Error)]
pub enum FixtureError {
    /// A local operation failed; its source stays attributed.
    #[error("fixture I/O failed during {phase}")]
    Io {
        /// The failed operation.
        phase: Phase,
        /// Original local failure, not an untrusted wire diagnostic.
        #[source]
        source: io::Error,
    },
    /// An individually named phase exceeded its unchanged deadline.
    #[error("fixture phase timed out ({phase})")]
    Timeout {
        /// The bounded operation.
        phase: Phase,
        /// Original elapsed deadline.
        #[source]
        source: tokio::time::error::Elapsed,
    },
    /// A bounded channel or transport ended.
    #[error("fixture closed during {0}")]
    Closed(Phase),
    /// A frame exceeded the single existing codec's limits or was invalid.
    #[error("fixture peer returned an invalid frame")]
    Protocol,
    /// A transcript or control capacity was reached before more work.
    #[error("fixture capacity is full")]
    Full,
    /// A scenario action was requested without a matching pending call.
    #[error("fixture has no matching pending call")]
    NoPendingCall,
    /// Checked fixture identities were exhausted.
    #[error("fixture identities exhausted")]
    Exhausted,
    /// The canonical actor refused admission.
    #[error("fixture admission could not be reserved")]
    Admission,
    /// A worker ended; panic payloads are not exposed.
    #[error("fixture worker ended")]
    Worker,
    /// Blocking a runtime inside another runtime would panic.
    #[error("fixture must be driven outside a Tokio runtime")]
    NestedRuntime,
    /// A scenario cannot be driven through another Link's queue/runtime.
    #[error("fixture belongs to another link")]
    WrongLink,
}
impl fmt::Debug for FixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub(super) async fn phase<T>(
    phase: Phase,
    work: impl Future<Output = T>,
) -> Result<T, FixtureError> {
    phase_with_limit(phase, super::DEADLINE, work).await
}

pub(super) async fn phase_with_limit<T>(
    phase: Phase,
    limit: std::time::Duration,
    work: impl Future<Output = T>,
) -> Result<T, FixtureError> {
    tokio::time::timeout(limit, Box::pin(work))
        .await
        .map_err(|source| FixtureError::Timeout { phase, source })
}
