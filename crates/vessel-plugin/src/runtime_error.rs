//! Supervisor diagnostics remain separate from scoped callback failures.

use crate::{runtime_types::CallError, wire::FrameError};

/// A closed protocol/state violation, never helper-authored text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProtocolViolation {
    /// No message may publish capability or complete work before ready.
    #[error("plugin helper sent an out-of-order message")]
    Ordering,
    /// The child may echo only ids the actor previously assigned.
    #[error("plugin helper replied to an unissued request")]
    UnissuedRequest,
    /// Counter exhaustion must not alias an earlier pipe correlation.
    #[error("plugin helper request identity exhausted")]
    IdentityExhausted,
    /// A bounded outbound queue cannot absorb necessary cancellation/control.
    #[error("plugin helper control queue is full")]
    ControlQueueFull,
}

/// Concrete host-side failures. A node handler maps their classes; it never
/// copies Display/source diagnostics into a scoped wire failure.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    /// Shutdown already revoked new helper starts.
    #[error("plugin helper owner is stopping")]
    Stopping,
    /// The public synchronous launch entry must refuse without a live reactor.
    #[error("plugin helper launch requires a Tokio runtime")]
    Runtime(#[source] tokio::runtime::TryCurrentError),
    /// Process runner retains platform/group cause.
    #[error("plugin helper could not start")]
    Spawn(#[source] vessel_proc::SpawnError),
    /// Framing errors already seal input-bearing parser details.
    #[error("plugin helper output failed")]
    Read(#[source] FrameError),
    /// A partially failed write retires the pipe, never retries a suffix.
    #[error("plugin helper input failed")]
    Write(#[source] FrameError),
    /// No complete initial declarations arrived within the outer budget.
    #[error("plugin helper initialization timed out")]
    ReadyTimeout,
    /// An admitted callback did not settle; group retirement releases admission.
    #[error("plugin helper callback timed out")]
    CallTimeout,
    /// A writer held the pipe beyond the bounded delivery budget.
    #[error("plugin helper pipe write timed out")]
    WriteTimeout,
    /// Explicit helper fatal text is retained only in private logs.
    #[error("plugin helper reported a fatal failure")]
    Fatal,
    /// Child lifetime ended unexpectedly, including a clean code without dispose.
    #[error("plugin helper exited ({code:?})")]
    Exited {
        /// OS exit code; no arbitrary child text.
        code: Option<i32>,
    },
    /// Stdout closed before the supervising shutdown.
    #[error("plugin helper closed its protocol pipe")]
    EndOfStream,
    /// A fixed protocol/state refusal.
    #[error("plugin helper violated its protocol")]
    Protocol(#[from] ProtocolViolation),
    /// Error reading the private diagnostic stream.
    #[error("plugin helper stderr failed")]
    Stderr(#[source] std::io::Error),
    /// A pipe retained by a descendant cannot hold lifecycle shutdown forever.
    #[error("plugin helper diagnostic drain timed out")]
    StderrDrainTimeout,
    /// No safe private log update is possible.
    #[error("plugin helper diagnostic store is unavailable")]
    Logs,
    /// Sequence cannot wrap into an older retained row.
    #[error("plugin helper diagnostic identity exhausted")]
    LogIdentity(#[source] crate::log_tail::LogExhausted),
    /// Group termination/reaping retains concrete OS causes.
    #[error("plugin helper group signal failed")]
    Signal(#[source] vessel_proc::SignalError),
    /// OS could not report the owned leader's status.
    #[error("plugin helper group wait failed")]
    Wait(#[source] vessel_proc::WaitError),
    /// Legacy leader status cannot hide the first group/job cleanup refusal.
    #[error("plugin helper owned cleanup could not be confirmed")]
    Cleanup(#[source] vessel_proc::CheckedWaitError),
    /// No successful disable/remove acknowledgement may follow this refusal.
    #[error("plugin helper group cleanup did not complete")]
    CleanupTimeout,
    /// A worker task failed; raw panic payloads remain sealed by `TaskFailure`.
    #[error("plugin helper worker failed")]
    Task(#[source] TaskFailure),
}

impl RuntimeError {
    pub(crate) const fn call_error(&self) -> CallError {
        match self {
            Self::ReadyTimeout | Self::CallTimeout | Self::WriteTimeout => CallError::Timeout,
            Self::Stopping
            | Self::Runtime(_)
            | Self::Spawn(_)
            | Self::Read(_)
            | Self::Write(_)
            | Self::Fatal
            | Self::Exited { .. }
            | Self::EndOfStream
            | Self::Protocol(_)
            | Self::Stderr(_)
            | Self::StderrDrainTimeout
            | Self::Logs
            | Self::LogIdentity(_)
            | Self::Signal(_)
            | Self::Wait(_)
            | Self::Cleanup(_)
            | Self::CleanupTimeout
            | Self::Task(_) => CallError::Closed,
        }
    }
}

/// Retains task cause without exposing an incidental panic payload in Debug or
/// a cause chain. The host itself must still be written without panic paths.
pub struct TaskFailure(tokio::task::JoinError);

impl TaskFailure {
    pub(crate) const fn new(error: tokio::task::JoinError) -> Self {
        Self(error)
    }
}

impl std::fmt::Debug for TaskFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for TaskFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_cancelled() {
            "plugin worker cancelled"
        } else {
            "plugin worker panicked"
        })
    }
}

impl std::error::Error for TaskFailure {}
