//! Files-only recovery values (ADR 0077). No I/O or provider message ids.
//!
//! Pending unary calls use Quality B's shared registration shape. Only integration
//! assigns a protocol. Conversation rewind is never a fallback.

mod bounds;
mod changes;
mod event;
mod evidence;
mod image;
mod metadata;
mod mutation;
mod owner;
mod rpc;

mod state;

pub use crate::id::{CaptureId, CheckpointId, RestoreId, RestorePlanId};
pub use bounds::{
    CHECKPOINT_CAPTURE_DEADLINE, CHECKPOINT_CAPTURE_RESERVATION_BYTES, CHECKPOINT_FILE_BYTES,
    CHECKPOINT_IMAGE_BYTES, CHECKPOINT_IO_CHUNK, CHECKPOINT_NODE_BYTES, CHECKPOINT_PAGE_ENTRIES,
    CHECKPOINT_PATH_BYTES, CHECKPOINT_PATHS, CHECKPOINT_PREVIEW_LIFETIME,
    CHECKPOINT_PREVIEWS_PER_OWNER, CHECKPOINT_QUEUE, CHECKPOINT_RESTORE_DEADLINE,
    CHECKPOINT_WORKERS, CHECKPOINTS_PER_SESSION, CheckpointFileBytes, CheckpointPath, ImageBytes,
    PathCount,
};
pub use changes::{Change, ChangeKind};
pub use event::CheckpointEvent;
pub use evidence::{
    FileFingerprint, IgnoreFingerprint, ObjectFingerprint, ObservedEntry, ObservedPresence,
    ObservedSnapshot, PhysicalRoot, RootFingerprint, SnapshotEvidence,
};
pub use image::{EffectiveDesiredImage, Entry, FileImage, FileState};
pub use metadata::{FileMetadata, GroupId, OrdinaryMode, OwnerId};
pub use mutation::{StagedPath, StagedPaths};
pub use owner::{CheckpointHumanOrigin, CheckpointOwner};
pub use rpc::{
    ChangeCursor, ChangePage, CheckpointCursor, CheckpointMark, CheckpointPage,
    CheckpointRestoreStatus, CheckpointState, ListCheckpoints, PreviewCheckpoint,
    ReadCheckpointChanges, RestoreCheckpoint, RestorePreview,
};
pub use state::{Boundary, CaptureGeneration, CaptureKey, CheckpointError, RestoreOutcome};

/// No peer may receive checkpoint calls before the integrator enables them.
/// Requires reviewed node, store, CLI/web and native consumers before assignment.
pub const FILE_CHECKPOINTS: Option<crate::ProtocolVersion> = None;

#[cfg(test)]
mod rpc_tests;
