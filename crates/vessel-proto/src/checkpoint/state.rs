//! Stable identities and outcomes; filenames or provider strings never select behavior.

use super::{CaptureId, PathCount, RestoreId};
use crate::{EventSeq, SessionId, WorktreeId};
use serde::{Deserialize, Serialize};

/// The two file restore meanings offered without altering conversation history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[serde(transparent)]
pub struct CaptureGeneration(u64);
impl CaptureGeneration {
    /// Initial lease for a coordinator incarnation.
    pub const FIRST: Self = Self(1);
    /// Exhaustion refuses another capture rather than reusing a stale fence.
    pub fn next(self) -> Result<Self, CheckpointError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(CheckpointError::GenerationExhausted)
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
#[serde(rename_all = "snake_case")]
pub enum CheckpointError {
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
