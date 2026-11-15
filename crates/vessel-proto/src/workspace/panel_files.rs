//! Bounded binary pages, duplicates and file observation (ADR 0083).
//! These values remain unregistered until the integration method table
//! assigns their shared feature a protocol number.

use base64::Engine as _;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use super::{ContentHash, RelPath, WorkspaceRoot};
use crate::ProtocolVersion;

/// Assigned only after node and native consumers are integrated.
pub const FILE_PANELS_PROTOCOL: Option<ProtocolVersion> = None;
/// Maximum source size for a binary transfer or directory duplication.
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum bytes in a single binary page (Paseo's stream chunk size).
pub const MAX_PAGE_BYTES: usize = 256 * 1024;
/// Encoded length checked before allocating a peer's decoded page.
const MAX_PAGE_BASE64_CHARS: usize = MAX_PAGE_BYTES.div_ceil(3) * 4;

/// A byte position in a file, independent of character positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct FileOffset(pub u64);

/// A source byte count, distinct from a position within that source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct FileSize(pub u64);

/// The exact content identity of a bounded file, also its transfer size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FileRevision {
    /// The number of source bytes, never a UTF-8 character count.
    pub size: FileSize,
    /// SHA-256 over the complete file, including binary bytes.
    pub hash: ContentHash,
}

/// A bounded opaque page; base64 on the wire, never interpreted as text.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct FileBytes(Bytes);

/// A peer supplied a page larger than the binary page budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a file page is at most {MAX_PAGE_BYTES} bytes")]
pub struct InvalidFileBytes;

impl FileBytes {
    /// Refuse an oversized page at its decoding/construction boundary.
    ///
    /// # Errors
    /// [`InvalidFileBytes`] for a page beyond [`MAX_PAGE_BYTES`].
    pub fn new(bytes: Bytes) -> Result<Self, InvalidFileBytes> {
        if bytes.len() > MAX_PAGE_BYTES {
            return Err(InvalidFileBytes);
        }
        Ok(Self(bytes))
    }

    /// Borrow the exact bytes without copying the page.
    #[must_use]
    pub const fn bytes(&self) -> &Bytes {
        &self.0
    }
}

impl Serialize for FileBytes {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::base64_bytes::serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for FileBytes {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(FileBytesVisitor)
    }
}

struct FileBytesVisitor;

impl serde::de::Visitor<'_> for FileBytesVisitor {
    type Value = FileBytes;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "at most {MAX_PAGE_BYTES} bytes encoded as base64"
        )
    }

    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Self::Value, E> {
        if text.len() > MAX_PAGE_BASE64_CHARS {
            return Err(E::custom(InvalidFileBytes));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(text)
            .map_err(E::custom)?;
        FileBytes::new(bytes.into()).map_err(E::custom)
    }
}

/// Read one page; later pages must name the first page's revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadFileBytes {
    /// Authorized workspace selection.
    pub root: WorkspaceRoot,
    /// Validated path below that root.
    pub path: RelPath,
    /// First byte requested; zero starts a transfer.
    pub offset: FileOffset,
    /// Required on continuation, optional only at offset zero.
    pub expected: Option<FileRevision>,
}

/// One exact page with a version-bound continuation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FileBytePage {
    /// Stable identity of the entire source.
    pub revision: FileRevision,
    /// Starting byte, echoed to detect misrouted/incomplete transfers.
    pub offset: FileOffset,
    /// Exact source bytes.
    pub bytes: FileBytes,
    /// Next byte when the source has more; absent at EOF.
    pub next: Option<FileOffset>,
}

/// Duplicate an entry to the first available sibling copy name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DuplicateFileEntry {
    /// Authorized workspace selection.
    pub root: WorkspaceRoot,
    /// Source entry; the workspace root itself is refused.
    pub path: RelPath,
}

/// The new sibling path; never an assumed local path on the client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DuplicatedFileEntry {
    /// The exclusively created copy.
    pub path: RelPath,
}

/// What an observed saved file currently contains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObservedFile {
    /// Complete content identity; a binary file is equally observable.
    Ready {
        /// The source revision.
        revision: FileRevision,
    },
    /// No file exists; clients preserve an unsaved buffer.
    Missing,
}

/// Observe until a change notification or a finite heartbeat deadline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ObserveFile {
    /// Authorized workspace selection.
    pub root: WorkspaceRoot,
    /// File path, never the workspace root.
    pub path: RelPath,
    /// The last state the client received; absent for an initial snapshot.
    pub after: Option<ObservedFile>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_pages_preserve_nuls_and_refuse_oversized_decodes() {
        let page = FileBytes::new(Bytes::from_static(b"\0\xffa")).unwrap();
        let encoded = serde_json::to_string(&page).unwrap();
        assert_eq!(serde_json::from_str::<FileBytes>(&encoded).unwrap(), page);
        assert!(FileBytes::new(Bytes::from(vec![0; MAX_PAGE_BYTES + 1])).is_err());
    }
}
