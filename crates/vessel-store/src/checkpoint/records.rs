//! Decoded durable checkpoint rows; SQL/JSON stay inside this crate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::checkpoint::{
    CaptureKey, CheckpointOwner, EffectiveDesiredImage, FileImage, RestoreId, RestoreOutcome,
    RestorePlanId, SnapshotEvidence,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Plan {
    pub(crate) id: RestorePlanId,
    pub(crate) key: CaptureKey,
    pub(crate) owner: CheckpointOwner,
    pub(crate) current: FileImage,
    pub(crate) evidence: SnapshotEvidence,
    pub(crate) desired: EffectiveDesiredImage,
    pub(crate) expires: DateTime<Utc>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Operation {
    pub(crate) session: vessel_proto::SessionId,
    pub(crate) id: RestoreId,
    pub(crate) plan: RestorePlanId,
    pub(crate) owner: CheckpointOwner,
    pub(crate) outcome: RestoreOutcome,
    pub(crate) before: FileImage,
}
/// Typed checkpoint storage failure; SQL/JSON sources never cross the RPC boundary.
#[derive(Debug, thiserror::Error)]
pub enum CheckpointStoreError {
    /// The named preview lifetime does not fit the clock's representation.
    #[error("checkpoint preview lifetime could not be represented")]
    Duration(#[from] chrono::OutOfRangeError),
    /// A stored operation id is malformed.
    #[error("checkpoint record contains an invalid operation identity")]
    Id(#[source] <RestoreId as std::str::FromStr>::Err),
    /// The database operation failed; retain the original driver source.
    #[error("checkpoint storage operation failed")]
    Sql(#[from] rusqlite::Error),
    /// The persistent typed record failed to encode or decode.
    #[error("checkpoint record could not be encoded or decoded")]
    Json(#[from] serde_json::Error),
    /// A valid request cannot proceed under the checkpoint contract.
    #[error("checkpoint operation refused: {0}")]
    Refused(#[from] vessel_proto::checkpoint::CheckpointError),
}

impl CheckpointStoreError {
    /// Classify once without exposing a source error's paths or serialized data.
    #[must_use]
    pub const fn code(&self) -> vessel_proto::ErrorCode {
        use vessel_proto::{ErrorCode, checkpoint::CheckpointError};
        match self {
            Self::Duration(_) | Self::Id(_) | Self::Sql(_) | Self::Json(_) => ErrorCode::Internal,
            Self::Refused(reason) => match reason {
                CheckpointError::FileTooLarge
                | CheckpointError::ImageTooLarge
                | CheckpointError::TooManyPaths
                | CheckpointError::InvalidPath
                | CheckpointError::InvalidCursor
                | CheckpointError::DuplicatePath => ErrorCode::InvalidRequest,
                CheckpointError::UnsupportedMetadata
                | CheckpointError::UnsupportedPlatform
                | CheckpointError::UnsupportedScope
                | CheckpointError::UnsafeEntry => ErrorCode::Unsupported,
                CheckpointError::Busy
                | CheckpointError::StaleCapture
                | CheckpointError::CaptureUnstable
                | CheckpointError::StalePreview
                | CheckpointError::RestoreIdConflict
                | CheckpointError::RecoveryPending => ErrorCode::Conflict,
                CheckpointError::FeatureUnavailable
                | CheckpointError::CaptureTimedOut
                | CheckpointError::GenerationExhausted
                | CheckpointError::StorageFull => ErrorCode::Unavailable,
                CheckpointError::NotFound => ErrorCode::NotFound,
                CheckpointError::Forbidden => ErrorCode::Forbidden,
                CheckpointError::Unknown => ErrorCode::Internal,
            },
        }
    }
}
