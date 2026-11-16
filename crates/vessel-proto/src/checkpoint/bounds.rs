//! One source of truth for admission limits; bounded values validate on decode.

use super::CheckpointError;
use crate::workspace::RelPath;
use serde::{Deserialize, Serialize};
use std::{fmt, time::Duration};

/// Maximum captured raw bytes per regular file.
pub const CHECKPOINT_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum raw byte sum in one image.
pub const CHECKPOINT_IMAGE_BYTES: u64 = 128 * 1024 * 1024;
/// Initial capture escrow includes candidate, retained image and metadata overhead.
/// Cleanup settles this down to the retained image charge; clients supply no size.
pub const CHECKPOINT_CAPTURE_RESERVATION_BYTES: u64 = CHECKPOINT_IMAGE_BYTES.saturating_mul(4);
/// Maximum covered entries in one image.
pub const CHECKPOINT_PATHS: usize = 2048;
/// Maximum bytes in one named path.
pub const CHECKPOINT_PATH_BYTES: usize = 1024;
/// Retained ready images per session, excluding pins.
pub const CHECKPOINTS_PER_SESSION: usize = 40;
/// Global archive, staging and journal reservation cap.
pub const CHECKPOINT_NODE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// Bounds actor acknowledgement, never uninterruptible filesystem work.
pub const CHECKPOINT_CAPTURE_DEADLINE: Duration = Duration::from_secs(30);
/// Streaming allocation bound.
pub const CHECKPOINT_IO_CHUNK: usize = 64 * 1024;
/// Bounded response entries per page.
pub const CHECKPOINT_PAGE_ENTRIES: usize = 100;
/// Maximum pinned previews per complete authenticated human principal.
pub const CHECKPOINT_PREVIEWS_PER_OWNER: usize = 4;
/// How long a confirmation plan remains usable.
pub const CHECKPOINT_PREVIEW_LIFETIME: Duration = Duration::from_secs(5 * 60);
/// Deadline before a restore enters recovery.
pub const CHECKPOINT_RESTORE_DEADLINE: Duration = Duration::from_secs(60);
/// Includes expired jobs until their I/O and cleanup actually finish.
pub const CHECKPOINT_WORKERS: usize = 2;
/// Pending jobs; a full queue refuses without spawning a waiter.
pub const CHECKPOINT_QUEUE: usize = 8;

macro_rules! bounded {
    ($name:ident, $raw:ty, $repr:literal, $limit:expr, $error:ident, $why:literal) => {
        #[doc = $why]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[serde(try_from = $repr, into = $repr)]
        pub struct $name($raw);
        impl $name {
            /// Refuse over-limit input before allocating or admitting work.
            ///
            /// # Errors
            /// The matching size/count refusal when the named bound is exceeded.
            pub fn new(value: $raw) -> Result<Self, CheckpointError> {
                if value > $limit {
                    Err(CheckpointError::$error)
                } else {
                    Ok(Self(value))
                }
            }
            /// Scalar only at a wire, filesystem or accounting edge.
            #[must_use]
            pub const fn get(self) -> $raw {
                self.0
            }
        }
        impl TryFrom<$raw> for $name {
            type Error = CheckpointError;
            fn try_from(value: $raw) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
        impl From<$name> for $raw {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}
bounded!(
    CheckpointFileBytes,
    u64,
    "u64",
    CHECKPOINT_FILE_BYTES,
    FileTooLarge,
    "Validated raw file size."
);
bounded!(
    ImageBytes,
    u64,
    "u64",
    CHECKPOINT_IMAGE_BYTES,
    ImageTooLarge,
    "Validated image byte sum."
);
bounded!(
    PathCount,
    usize,
    "usize",
    CHECKPOINT_PATHS,
    TooManyPaths,
    "Validated image entry count."
);

/// A nonroot portable file path subject to checkpoint's tighter byte limit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "RelPath", into = "RelPath")]
pub struct CheckpointPath(RelPath);
impl CheckpointPath {
    /// Borrow the existing path type for root-relative filesystem access.
    #[must_use]
    pub fn relative(&self) -> &RelPath {
        &self.0
    }
}
impl TryFrom<RelPath> for CheckpointPath {
    type Error = CheckpointError;
    fn try_from(path: RelPath) -> Result<Self, Self::Error> {
        if path.is_root() || path.as_str().len() > CHECKPOINT_PATH_BYTES {
            Err(CheckpointError::InvalidPath)
        } else {
            Ok(Self(path))
        }
    }
}
impl From<CheckpointPath> for RelPath {
    fn from(value: CheckpointPath) -> Self {
        value.0
    }
}
impl fmt::Display for CheckpointPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_are_checked_on_decode_not_only_in_callers() {
        assert!(
            serde_json::from_str::<CheckpointFileBytes>(
                &CHECKPOINT_FILE_BYTES.saturating_add(1).to_string()
            )
            .is_err()
        );
        assert!(CheckpointFileBytes::new(CHECKPOINT_FILE_BYTES).is_ok());
        assert!(PathCount::new(CHECKPOINT_PATHS.saturating_add(1)).is_err());
        assert!(serde_json::from_str::<CheckpointPath>("\"\"").is_err());
        assert!(serde_json::from_str::<CheckpointPath>("\"../token\"").is_err());
    }
}
