//! Files-only recovery values (ADR 0077). No I/O or provider message ids.
//!
//! Calls remain unregistered until Quality B assigns their shared shape and
//! integration assigns a protocol. Conversation rewind is never a fallback.

mod bounds;
mod image;
mod metadata;
mod state;

pub use crate::id::{CaptureId, CheckpointId, RestoreId, RestorePlanId};
pub use bounds::{
    CHECKPOINT_CAPTURE_DEADLINE, CHECKPOINT_FILE_BYTES, CHECKPOINT_IMAGE_BYTES,
    CHECKPOINT_IO_CHUNK, CHECKPOINT_NODE_BYTES, CHECKPOINT_PAGE_ENTRIES, CHECKPOINT_PATH_BYTES,
    CHECKPOINT_PATHS, CHECKPOINT_PREVIEW_LIFETIME, CHECKPOINT_PREVIEWS_PER_DEVICE,
    CHECKPOINT_QUEUE, CHECKPOINT_RESTORE_DEADLINE, CHECKPOINT_WORKERS, CHECKPOINTS_PER_SESSION,
    CheckpointPath, FileBytes, ImageBytes, PathCount,
};
pub use image::{Entry, FileImage, FileState};
pub use metadata::{FileMetadata, GroupId, OrdinaryMode, OwnerId};
pub use state::{Boundary, CaptureGeneration, CaptureKey, CheckpointError, RestoreOutcome};

/// No peer may receive checkpoint calls before the integrator enables them.
/// Requires reviewed node, store, CLI/web and native consumers before assignment.
pub const FILE_CHECKPOINTS: Option<crate::ProtocolVersion> = None;
