//! Decoded durable checkpoint rows; SQL/JSON stay inside this crate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::checkpoint::{
    CaptureKey, CheckpointOwner, EffectiveDesiredImage, FileImage, RestoreId, RestoreOutcome,
    RestorePlanId, SnapshotEvidence,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Immutable preview evidence and effective target; this is not authentication.
/// The node derives this under root admission; the store reloads it at admission.
pub struct Plan {
    /// Immutable preview or restore identity.
    pub id: RestorePlanId,
    /// Exact capture boundary and generation.
    pub key: CaptureKey,
    /// Full authenticated human principal, including delegated origin.
    pub owner: CheckpointOwner,
    /// Whole before-image used by the preview.
    pub current: FileImage,
    /// Root, ignore policy and stable object evidence at preview time.
    pub evidence: SnapshotEvidence,
    /// Frozen effective image after applying the metadata policy.
    pub desired: EffectiveDesiredImage,
    /// Absolute preview expiry.
    pub expires: DateTime<Utc>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// A snapshot of one durable restore; retries reload its authoritative row.
pub struct Operation {
    /// Session owning the restore, independent of any current runtime.
    pub session: vessel_proto::SessionId,
    /// Immutable preview or restore identity.
    pub id: RestoreId,
    /// Consumed preview identity.
    pub plan: RestorePlanId,
    /// Full authenticated human principal, including delegated origin.
    pub owner: CheckpointOwner,
    /// Durable result as read from this operation snapshot.
    pub outcome: RestoreOutcome,
    /// Retained pre-restore image for verification/recovery.
    pub before: FileImage,
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
