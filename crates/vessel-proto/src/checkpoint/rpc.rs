//! Files-only RPC metadata. Archive contents and filesystem roots never cross this edge.
//!
//! [Decision Log]
//! - Purpose: preview and confirmation use the same server-owned immutable plan.
//! - Constraint: conversation rewind cannot restore provider-independent file images.
//! - Choice: five unary calls, gated by the unassigned feature version; pagination
//!   carries an identity or bounded offset and never a client-supplied file path.
//! - Cost: the UI must finish every changes page before enabling confirmation.

use super::{
    Boundary, CaptureGeneration, CaptureId, Change, CheckpointError, CheckpointId, PathCount,
    RestoreId, RestorePlanId,
};
use crate::SessionId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Keyset position in one session's capture metadata; never a filesystem path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(type = "string"))]
#[serde(transparent)]
pub struct CheckpointCursor(pub CaptureId);

/// An offset within the frozen, bounded change list of the named plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(type = "number"))]
#[serde(transparent)]
pub struct ChangeCursor(pub PathCount);

/// `checkpoint.list`: metadata only, including unavailable and expired boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListCheckpoints {
    /// Session visibility is checked before any cursor lookup.
    pub session: SessionId,
    /// Omit for the newest page; the node binds the cursor to this session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<CheckpointCursor>,
}

/// A missing image is explicit and can never be synthesized from today's files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CheckpointState {
    /// A durable, immutable image is available.
    Ready {
        /// The selected image identity.
        checkpoint: CheckpointId,
    },
    /// Capture failed or could not be admitted.
    Unavailable {
        /// The stable, redacted reason.
        reason: CheckpointError,
    },
    /// Retention removed the unpinned image.
    Expired,
}

/// One turn's file boundary, independent of provider message identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CheckpointMark {
    /// Exact persisted prompt or turn end.
    pub boundary: Boundary,
    /// Prevents a late capture attaching to a later boundary incarnation.
    pub generation: CaptureGeneration,
    /// Availability cannot be inferred from a file-change count.
    pub state: CheckpointState,
}

/// At most `CHECKPOINT_PAGE_ENTRIES` marks; continuation is session-bound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CheckpointPage {
    /// Newest first, with deterministic capture-id ordering for ties.
    pub marks: Vec<CheckpointMark>,
    /// Present only when another bounded page exists.
    pub next: Option<CheckpointCursor>,
}

/// `checkpoint.preview`: pin a plan before replying; no project writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PreviewCheckpoint {
    /// Session to which the selected image must belong.
    pub session: SessionId,
    /// A ready local image, never a historical provider anchor alone.
    pub checkpoint: CheckpointId,
}

/// One bounded page of the same frozen effective desired image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChangePage {
    /// Identity binds every page to the authorization shown in the dialog.
    pub plan: RestorePlanId,
    /// Shared expiry, including every subsequent page.
    pub expires: DateTime<Utc>,
    /// Sorted path effects, at most `CHECKPOINT_PAGE_ENTRIES`.
    pub changes: Vec<Change>,
    /// Omit only after the full change list is delivered.
    pub next: Option<ChangeCursor>,
}

/// Frozen confirmation metadata; raw captured access bits are not replayed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RestorePreview {
    /// Selected local boundary.
    pub boundary: Boundary,
    /// Lease incarnation of the selected image.
    pub generation: CaptureGeneration,
    /// Total effects; the app must collect all of them before confirmation.
    pub changed: PathCount,
    /// First page; its plan and expiry apply to the entire preview.
    pub page: ChangePage,
}

/// `checkpoint.changes`: plan ownership is checked before reading any paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadCheckpointChanges {
    /// Opaque server-side authorization record.
    pub plan: RestorePlanId,
    /// Omit to repeat the first page safely after a connection failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<ChangeCursor>,
}

/// `checkpoint.restore`: retry the same id after a lost response, never reapply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RestoreCheckpoint {
    /// Session and plan must match their persisted association.
    pub session: SessionId,
    /// Exact preview confirmed by the human device.
    pub plan: RestorePlanId,
    /// Client-minted idempotency identity, checked before expiry on retries.
    pub restore: RestoreId,
}

/// `checkpoint.restore_status`: query the durable outcome after reconnecting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CheckpointRestoreStatus {
    /// Must match the operation even when its id is known.
    pub session: SessionId,
    /// The same identity supplied to restore admission.
    pub restore: RestoreId,
}
