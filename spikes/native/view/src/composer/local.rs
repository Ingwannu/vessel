//! Local byte identities never alias node attachment/upload identities.
//! Only validated keys form managed paths; display filenames are metadata.
use serde::{Deserialize, Serialize};

/// A key is one bounded ASCII path component, independent of its display name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LocalKey(String);

/// Local metadata is rejected before any managed-directory path is opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LocalProblem {
    /// Empty, long or path-like ids are never used for filesystem operations.
    #[error("invalid local attachment key")]
    Key,
    /// Empty bytes and files over the existing generic upload bound are refused.
    #[error("invalid local attachment byte length")]
    Size,
}

impl LocalKey {
    /// Maximum managed key length, independent of the original filename.
    pub const MAX: usize = 64;
    /// Validate a single component without dots, slashes, whitespace or controls.
    ///
    /// # Errors
    /// Returns `LocalProblem::Key` for any invalid key.
    pub fn parse(value: &str) -> Result<Self, LocalProblem> {
        if value.is_empty()
            || value.len() > Self::MAX
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            return Err(LocalProblem::Key);
        }
        Ok(Self(value.to_owned()))
    }
    /// The confined key; it is never the source filename or a URL.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for LocalKey {
    type Error = LocalProblem;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}
impl From<LocalKey> for String {
    fn from(key: LocalKey) -> Self {
        key.0
    }
}

/// The immutable length admitted before copying and checked after writing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct LocalSize(u64);
impl LocalSize {
    /// Reuse the node's generic file limit instead of a second upload policy.
    pub const MAX: u64 = vessel_proto::AttachmentLimits::DEFAULT.file;
    /// Admit only a nonempty file within that bound.
    ///
    /// # Errors
    /// Returns `LocalProblem::Size` for zero or oversized files.
    pub fn new(value: u64) -> Result<Self, LocalProblem> {
        if value == 0 || value > Self::MAX {
            return Err(LocalProblem::Size);
        }
        Ok(Self(value))
    }
    /// The exact expected number of bytes.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}
impl TryFrom<u64> for LocalSize {
    type Error = LocalProblem;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<LocalSize> for u64 {
    fn from(size: LocalSize) -> Self {
        size.0
    }
}

/// A persisted byte record carries no arbitrary filesystem path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalRecord {
    /// Managed storage identity.
    pub key: LocalKey,
    /// Sanitized display name, never used to resolve a path.
    pub name: vessel_proto::AttachmentName,
    /// Wire content type, validated by the existing protocol type.
    pub mime: vessel_proto::conversation::MimeType,
    /// Bounded immutable length.
    pub size: LocalSize,
    /// Creation time for retention grace, rather than source mtime.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// An opened local file holds its metadata and managed path together.
/// Runtime owners retain this by Arc; the store protects those leases from GC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFile {
    /// Validated persistent metadata.
    pub record: LocalRecord,
    /// The store constructed this path beneath its own private root.
    pub path: std::path::PathBuf,
}

#[cfg(test)]
mod tests;
