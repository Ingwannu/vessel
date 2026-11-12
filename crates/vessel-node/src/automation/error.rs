//! What the automation calls refuse with, mapped to the wire once (AGENTS.md
//! section 6): kept here rather than in the node's `ServiceError`, which
//! other tracks of the milestone grow at the same time.

use vessel_proto::{ErrorCode, WireError};

/// Why a schedule, saved prompt or notice call was refused.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AutomationError {
    /// No schedule, saved prompt or hook by that id.
    #[error("no such {0}")]
    NotFound(&'static str),
    /// The caller may not do this.
    #[error("{0}")]
    Forbidden(String),
    /// It is in a state that does not allow it, or a bound is reached.
    #[error("{0}")]
    Conflict(String),
    /// What was asked does not hold together.
    #[error("{0}")]
    Invalid(String),
    /// The node cannot do it now; later may work.
    #[error("{0}")]
    Unavailable(String),
    /// The store failed.
    #[error("the node's store failed: {0}")]
    Store(#[from] vessel_store::StoreError),
}

impl AutomationError {
    /// The error on the wire.
    pub(crate) fn to_wire(&self) -> WireError {
        let code = match self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Forbidden(_) => ErrorCode::Forbidden,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::Invalid(_) => ErrorCode::InvalidRequest,
            Self::Unavailable(_) => ErrorCode::Unavailable,
            Self::Store(_) => ErrorCode::Internal,
        };
        WireError::new(code, self.to_string())
    }
}
