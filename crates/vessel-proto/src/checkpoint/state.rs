//! Stable identities and outcomes; filenames or provider strings never select behavior.

use super::{CaptureId, PathCount, RestoreId};
use crate::{EventSeq, SessionId, WorktreeId};
use serde::{Deserialize, Serialize};

/// The two file restore meanings offered without altering conversation history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "CheckpointBoundary"))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Boundary {
    /// Before a new turn's accepted prompt reaches the agent.
    BeforeTurn {
        /// The prompt's durable log position.
        prompt: EventSeq,
    },
    /// After the completed turn's log record is persisted.
    AfterTurn {
        /// The durable turn-ended position.
        ended: EventSeq,
    },
}
/// A monotonic capture lease; no sentinel/wrapping generation is valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "u64", into = "u64")]
pub struct CaptureGeneration(u64);
impl CaptureGeneration {
    /// Exact integer ceiling shared by `SQLite` persistence and JavaScript peers.
    /// Going beyond it would round a fence on the wire instead of rejecting it.
    pub const MAX: Self = Self(9_007_199_254_740_991);
    /// Initial lease for a coordinator incarnation.
    pub const FIRST: Self = Self(1);
    /// Exhaustion refuses another capture rather than reusing a stale fence.
    ///
    /// # Errors
    /// `GenerationExhausted` when the monotonic counter cannot advance.
    pub fn next(self) -> Result<Self, CheckpointError> {
        self.0
            .checked_add(1)
            .ok_or(CheckpointError::GenerationExhausted)
            .and_then(Self::try_from)
    }
}
impl TryFrom<u64> for CaptureGeneration {
    type Error = CheckpointError;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value == 0 {
            Err(CheckpointError::StaleCapture)
        } else if value > Self::MAX.0 {
            Err(CheckpointError::GenerationExhausted)
        } else {
            Ok(Self(value))
        }
    }
}
impl From<CaptureGeneration> for u64 {
    fn from(value: CaptureGeneration) -> Self {
        value.0
    }
}
/// Immutable association carried on every candidate completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureKey {
    /// Unique candidate staging namespace.
    pub capture: CaptureId,
    /// Session whose files this image can restore.
    pub session: SessionId,
    /// Registered workspace identity, never a client-supplied root path.
    pub worktree: WorktreeId,
    /// Exact boundary; a steer is never a new capture.
    pub boundary: Boundary,
    /// Coordinator fence; late results cannot attach to another request.
    pub generation: CaptureGeneration,
}
/// Stable typed refusal; messages derive all bounds from their named constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CheckpointError {
    /// Registration precedes activation; no incomplete mechanism may write files.
    #[error("file checkpoint service is not enabled")]
    FeatureUnavailable,
    /// A cursor belongs to another snapshot or exceeds its page domain.
    #[error("checkpoint cursor is invalid")]
    InvalidCursor,
    /// File exceeded its declared streaming budget.
    #[error("file exceeds {} bytes", super::CHECKPOINT_FILE_BYTES)]
    FileTooLarge,
    /// Image admission failed before a ready publication.
    #[error("image exceeds {} bytes", super::CHECKPOINT_IMAGE_BYTES)]
    ImageTooLarge,
    /// Full enumeration cannot fit.
    #[error("image exceeds {} paths", super::CHECKPOINT_PATHS)]
    TooManyPaths,
    /// Root/path shape is not part of this feature's coverage.
    #[error(
        "checkpoint path must be nonroot and at most {} bytes",
        super::CHECKPOINT_PATH_BYTES
    )]
    InvalidPath,
    /// Duplicate names would make restore ordering ambiguous.
    #[error("checkpoint repeats a path")]
    DuplicatePath,
    /// Privileged/unsafe metadata cannot be honored.
    #[error("file metadata cannot be preserved safely")]
    UnsupportedMetadata,
    /// Initial implementation deliberately refuses unproven platform semantics.
    #[error("file checkpoints are unsupported on this platform")]
    UnsupportedPlatform,
    /// Ordinary projects/historical/terminal sessions have no image.
    #[error("session has no supported managed worktree")]
    UnsupportedScope,
    /// Link, special file or filesystem boundary.
    #[error("checkpoint contains an unsafe entry")]
    UnsafeEntry,
    /// Bounded admission is occupied, including expired but uncleaned workers.
    #[error("checkpoint admission is busy")]
    Busy,
    /// Actor wait is over; this says nothing about kernel I/O cancellation.
    #[error("checkpoint acknowledgement timed out")]
    CaptureTimedOut,
    /// Candidate identity or lease does not match.
    #[error("capture completion is stale")]
    StaleCapture,
    /// No monotonic generation remains.
    #[error("capture generation exhausted")]
    GenerationExhausted,
    /// A preflight image changed while being read.
    #[error("files changed while capturing")]
    CaptureUnstable,
    /// Space includes reserved staging/recovery, not only published blobs.
    #[error("checkpoint storage is full")]
    StorageFull,
    /// Confirmation no longer authorizes the current files.
    #[error("restore preview is stale")]
    StalePreview,
    /// An id cannot be reused for a different authorization/plan.
    #[error("restore id conflicts with an existing operation")]
    RestoreIdConflict,
    /// Journal work has started and needs reconciliation.
    #[error("workspace recovery is pending")]
    RecoveryPending,
    /// Required persistent row does not exist.
    #[error("checkpoint operation not found")]
    NotFound,
    /// Closed devices or agent principals cannot confirm file restores.
    #[error("checkpoint operation is forbidden")]
    Forbidden,
    /// Unknown newer-peer failure is preserved at the decoding edge.
    #[serde(other)]
    #[error("unknown checkpoint refusal")]
    Unknown,
}
/// Files-only result: a failed multi-file operation is never reported as success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RestoreOutcome {
    /// Durable admission lets a retry query progress without reapplying files.
    InProgress {
        /// Idempotency identity.
        restore: RestoreId,
    },
    /// All effective targets verified and result committed.
    Restored {
        /// Number of approved changed paths.
        changed: PathCount,
    },
    /// Current image already equals the effective target.
    Unchanged,
    /// Verified recovery to the pre-restore image.
    RolledBack {
        /// Typed reason the requested operation did not finish.
        reason: CheckpointError,
    },
    /// Before image and journal stay pinned for operator reconciliation.
    RecoveryRequired {
        /// Query this operation instead of retrying blindly.
        restore: RestoreId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checkpoint_generation_never_rounds_or_overflows_its_persistent_wire_fence() {
        assert_eq!(
            CaptureGeneration::try_from(0),
            Err(CheckpointError::StaleCapture)
        );
        assert_eq!(
            CaptureGeneration::MAX.next(),
            Err(CheckpointError::GenerationExhausted)
        );
        let maximum = u64::from(CaptureGeneration::MAX);
        assert_eq!(
            CaptureGeneration::try_from(maximum),
            Ok(CaptureGeneration::MAX)
        );
        assert_eq!(
            CaptureGeneration::try_from(maximum.checked_add(1).unwrap()),
            Err(CheckpointError::GenerationExhausted)
        );
        assert!(serde_json::from_str::<CaptureGeneration>(&u64::MAX.to_string()).is_err());
        assert_eq!(
            serde_json::from_str::<CaptureGeneration>(&maximum.to_string()).unwrap(),
            CaptureGeneration::MAX
        );
    }
}
