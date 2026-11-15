//! Captured metadata versus effective restore metadata: never widen current access.

use super::CheckpointError;
use serde::{Deserialize, Serialize};

/// Unix ordinary mode mask, deliberately excluding setuid/setgid/sticky bits.
const ORDINARY_BITS: u32 = 0o777;
const EXECUTE_BITS: u32 = 0o111;
const READ_WRITE_BITS: u32 = 0o666;
const PRIVATE_READ_WRITE_BITS: u32 = 0o600;

macro_rules! account_id {
    ($name:ident, $why:literal) => {
        #[doc = $why]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(u32);
        impl $name {
            /// Wrap a kernel-supplied identity at the filesystem edge.
            #[must_use]
            pub const fn new(value: u32) -> Self {
                Self(value)
            }
            /// Raw identity at the platform edge only.
            #[must_use]
            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}
account_id!(OwnerId, "A file owner, never a group or session id.");
account_id!(GroupId, "A file group, never an owner or session id.");

/// Only ordinary Unix permissions can be replayed, with policy applied first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct OrdinaryMode(u32);
impl OrdinaryMode {
    /// Reject privileged or unknown mode bits rather than strip them silently.
    ///
    /// # Errors
    /// `UnsupportedMetadata` when nonordinary mode bits are present.
    pub fn new(value: u32) -> Result<Self, CheckpointError> {
        if value & !ORDINARY_BITS != 0 {
            Err(CheckpointError::UnsupportedMetadata)
        } else {
            Ok(Self(value))
        }
    }
    /// Use only when applying permissions or comparing platform metadata.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
    /// Keep current read/write bits and take only captured executable bits.
    #[must_use]
    pub const fn for_existing(self, current: Self) -> Self {
        Self((current.0 & READ_WRITE_BITS) | (self.0 & EXECUTE_BITS))
    }
    /// A recreated file is private regardless of historical read/write bits.
    #[must_use]
    pub const fn for_absent(self) -> Self {
        Self(PRIVATE_READ_WRITE_BITS | (self.0 & EXECUTE_BITS))
    }
}
impl TryFrom<u32> for OrdinaryMode {
    type Error = CheckpointError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<OrdinaryMode> for u32 {
    fn from(value: OrdinaryMode) -> Self {
        value.0
    }
}

/// Metadata in an immutable image; ownership is evidence, never blindly replayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Required to retain an existing file's owner.
    pub owner: OwnerId,
    /// Required to retain an existing file's group.
    pub group: GroupId,
    /// Ordinary permissions only.
    pub mode: OrdinaryMode,
}
impl FileMetadata {
    /// Freeze the effective target, used by preview, journal and verification.
    #[must_use]
    pub fn effective(self, current: Option<Self>, account: Self) -> Self {
        match current {
            Some(current) => Self {
                mode: self.mode.for_existing(current.mode),
                ..current
            },
            None => Self {
                mode: self.mode.for_absent(),
                ..account
            },
        }
    }
}
