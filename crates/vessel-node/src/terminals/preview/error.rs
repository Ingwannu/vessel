//! Fixed typed refusals keep system causes without page-selected diagnostics.

use tokio::sync::TryAcquireError;
use vessel_proto::workspace::InvalidServicePort;

use super::PreviewBudget;

/// Missing dependencies never turn metadata into a private-origin permission.
#[derive(Debug, Clone, Copy)]
pub(crate) enum MissingPreviewProof {
    CanonicalAuthority,
    RunIdentity,
    SocketPeer,
    Scheme,
    Platform,
}

impl std::fmt::Display for MissingPreviewProof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CanonicalAuthority => "canonical workspace authority",
            Self::RunIdentity => "actual service-run identity",
            Self::SocketPeer => "connected service ownership",
            Self::Scheme => "verified service scheme",
            Self::Platform => "connected service ownership on this platform",
        })
    }
}

/// This internal result is projected once by the authorized browser adapter.
#[derive(Debug, thiserror::Error)]
pub(crate) enum PreviewError {
    #[error("service Preview cannot verify {0}")]
    Unavailable(MissingPreviewProof),
    #[error("service Preview requires a literal loopback origin")]
    NotLoopback,
    #[error("service Preview requires a TCP service port")]
    Port(#[source] InvalidServicePort),
    #[error("service Preview belongs to another workspace or opening session")]
    Scope,
    #[error("service Preview belongs to another connection")]
    Connection,
    #[error("service Preview does not permit that exact origin")]
    Origin,
    #[error("service Preview ownership or run changed")]
    Changed,
    #[error("service Preview evidence expired")]
    Expired,
    #[error("service Preview evidence has an invalid observation time")]
    Clock,
    #[error("service Preview was revoked")]
    Revoked,
    #[error("service Preview {budget} capacity of {limit} is busy", limit = budget.limit())]
    Busy {
        budget: PreviewBudget,
        #[source]
        source: TryAcquireError,
    },
    #[error("service Preview connection failed")]
    Socket(#[source] std::io::Error),
    #[error("service Preview inspection did not finish")]
    Worker(#[source] tokio::task::JoinError),
    #[error("service Preview operation exceeded its budget")]
    Deadline(#[source] tokio::time::error::Elapsed),
}
