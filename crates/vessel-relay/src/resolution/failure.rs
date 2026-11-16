//! Fixed public categories with privately retained, input-free source wrappers.

use std::{fmt, io, sync::Arc};

/// Why resolution or a normal drain could not complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// No finite admission slot is available; no work was spawned.
    Full,
    /// This generation has closed admission.
    Closed,
    /// Input is empty, oversized or includes URL/control components.
    InvalidHost,
    /// A scope's quota is zero or exceeds the lifecycle root's budget.
    InvalidQuota,
    /// The resolver produced no usable records.
    Empty,
    /// The whole result exceeds the raw-record bound.
    TooManyAddresses,
    /// Standard native resolution failed.
    Native,
    /// Worker/join ownership could not establish normal completion.
    Worker,
}

impl fmt::Display for FailureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Full => "resolution capacity is full",
            Self::Closed => "resolution scope is closed",
            Self::InvalidHost => "invalid resolution host",
            Self::InvalidQuota => "invalid resolution quota",
            Self::Empty => "resolution returned no addresses",
            Self::TooManyAddresses => "resolution returned too many addresses",
            Self::Native => "native resolution failed",
            Self::Worker => "resolution worker did not complete normally",
        })
    }
}

/// A safe error chain. Original native/worker causes remain private; formatting
/// or following source() cannot expose a host, panic payload or OS diagnostic.
#[derive(Clone, thiserror::Error)]
#[error("{kind}")]
pub struct Failure {
    kind: FailureKind,
    #[source]
    cause: Option<Arc<Cause>>,
}

impl Failure {
    pub(super) fn new(kind: FailureKind) -> Self {
        Self { kind, cause: None }
    }

    pub(super) fn native(error: io::Error) -> Self {
        Self {
            kind: FailureKind::Native,
            cause: Some(Arc::new(Cause::Native(error))),
        }
    }

    pub(super) fn worker(error: tokio::task::JoinError) -> Self {
        Self {
            kind: FailureKind::Worker,
            cause: Some(Arc::new(Cause::Worker(error))),
        }
    }

    pub(super) fn runtime(error: tokio::runtime::TryCurrentError) -> Self {
        Self {
            kind: FailureKind::Worker,
            cause: Some(Arc::new(Cause::Runtime(error))),
        }
    }

    pub(super) fn receiver(error: tokio::sync::oneshot::error::RecvError) -> Self {
        Self {
            kind: FailureKind::Worker,
            cause: Some(Arc::new(Cause::Receiver(error))),
        }
    }

    /// Closed classification, without a diagnostic string.
    #[must_use]
    pub const fn kind(&self) -> FailureKind {
        self.kind
    }
}

impl fmt::Debug for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolutionFailure")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

enum Cause {
    Native(io::Error),
    Worker(tokio::task::JoinError),
    Runtime(tokio::runtime::TryCurrentError),
    Receiver(tokio::sync::oneshot::error::RecvError),
}

impl fmt::Debug for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Native(error) => f.debug_tuple("NativeCause").field(&error.kind()).finish(),
            Self::Worker(error) => f
                .debug_struct("WorkerCause")
                .field("cancelled", &error.is_cancelled())
                .field("panicked", &error.is_panic())
                .finish(),
            Self::Runtime(error) => f.debug_tuple("RuntimeCause").field(error).finish(),
            Self::Receiver(error) => f
                .debug_tuple("CompletionReceiverCause")
                .field(error)
                .finish(),
        }
    }
}
impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Native(_) => "native resolver cause",
            Self::Worker(_) => "worker completion cause",
            Self::Runtime(_) => "resolution runtime unavailable",
            Self::Receiver(_) => "resolution completion unavailable",
        })
    }
}
impl std::error::Error for Cause {}
