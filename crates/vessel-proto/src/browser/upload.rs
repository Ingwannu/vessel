//! Upload names are transported without authorizing a host filesystem read.
//!
//! The engine must resolve/stage regular files under the authenticated session's
//! workspace. Parsing a path here neither checks existence nor follows links.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize};

use super::arguments::ElementArgs;
use crate::workspace::RelPath;

/// Request decoding and the future node-wide retained staging ledger share this
/// file-count admission ceiling (ADR0078). This value creates no staging authority.
pub const MAX_UPLOAD_FILES: u32 = 32;

/// Upload input refusals do not reflect a path that may contain private data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidUpload {
    /// Empty path lists cannot represent an upload intent.
    #[error("browser upload requires at least one file")]
    EmptyList,
    /// Also bounds retained manifest entries when the staging owner is wired.
    #[error("browser upload exceeds {MAX_UPLOAD_FILES} files")]
    FileCount,
    /// An empty or NUL-containing path cannot name a regular file.
    #[error("upload path must be nonempty, contain no NUL and fit {max} UTF-8 bytes", max = RelPath::MAX_BYTES)]
    Path,
}

/// A file name, still untrusted until workspace confinement/staging succeeds.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct UploadPath(Arc<str>);

impl UploadPath {
    /// Filesystem validation receives a path type rather than a command string.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        Path::new(self.0.as_ref())
    }
}

impl TryFrom<String> for UploadPath {
    type Error = InvalidUpload;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() || value.len() > RelPath::MAX_BYTES || value.contains('\0') {
            Err(InvalidUpload::Path)
        } else {
            Ok(Self(Arc::from(value)))
        }
    }
}

impl From<UploadPath> for String {
    fn from(value: UploadPath) -> Self {
        value.0.to_string()
    }
}

impl fmt::Debug for UploadPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UploadPath([untrusted])")
    }
}

/// Nonempty file intent; only the engine may turn these into staged file handles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UploadFiles(Vec<UploadPath>);

impl UploadFiles {
    /// The MCP JSON edge already has a raw array; reject its count before copying
    /// or parsing members. The constructor reuses this same checked boundary.
    /// # Errors
    /// An empty or over-budget intent cannot enter source/staging preparation.
    pub fn check_count(count: usize) -> Result<(), InvalidUpload> {
        if count == 0 {
            Err(InvalidUpload::EmptyList)
        } else if u32::try_from(count).map_or(true, |count| count > MAX_UPLOAD_FILES) {
            Err(InvalidUpload::FileCount)
        } else {
            Ok(())
        }
    }
    /// Borrow every file before performing any stage/attach side effect.
    #[must_use]
    pub fn as_slice(&self) -> &[UploadPath] {
        &self.0
    }
}

impl TryFrom<Vec<UploadPath>> for UploadFiles {
    type Error = InvalidUpload;
    fn try_from(value: Vec<UploadPath>) -> Result<Self, Self::Error> {
        Self::check_count(value.len())?;
        Ok(Self(value))
    }
}

impl<'de> Deserialize<'de> for UploadFiles {
    fn deserialize<D: Deserializer<'de>>(source: D) -> Result<Self, D::Error> {
        let files = super::lists::decode::<D, UploadPath, MAX_UPLOAD_FILES>(source)?;
        Self::try_from(files).map_err(serde::de::Error::custom)
    }
}

impl From<UploadFiles> for Vec<UploadPath> {
    fn from(value: UploadFiles) -> Self {
        value.0
    }
}

/// The actor validates the entire list before the browser can read any file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UploadArgs {
    /// The current input element reference belongs to the authorized target.
    pub target: ElementArgs,
    /// All names remain untrusted, even when lexically under a workspace.
    pub files: UploadFiles,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_collection_cannot_hide_an_empty_or_invalid_member() {
        assert!(serde_json::from_str::<UploadFiles>("[]").is_err());
        assert!(serde_json::from_str::<UploadFiles>(r#"["good", ""]"#).is_err());
        assert!(serde_json::from_str::<UploadFiles>(r#"["bad\u0000path"]"#).is_err());
        let paths: UploadFiles = serde_json::from_str(r#"["../still-untrusted"]"#).unwrap();
        assert_eq!(paths.as_slice().len(), 1);
        assert!(!format!("{paths:?}").contains("../still-untrusted"));
    }

    #[test]
    fn upload_count_refuses_before_decoding_the_first_excess_path() {
        let limit = usize::try_from(MAX_UPLOAD_FILES).unwrap();
        let accepted = vec![UploadPath::try_from("still-untrusted".to_owned()).unwrap(); limit];
        assert!(UploadFiles::try_from(accepted.clone()).is_ok());
        let mut excess = accepted;
        excess.push(UploadPath::try_from("another-untrusted".to_owned()).unwrap());
        assert!(matches!(
            UploadFiles::try_from(excess),
            Err(InvalidUpload::FileCount)
        ));
        let mut items = vec![r#""still-untrusted""#; limit];
        assert!(serde_json::from_str::<UploadFiles>(&format!("[{}]", items.join(","))).is_ok());
        items.push(r#"{"do_not_decode_this_path":true}"#);
        let error =
            serde_json::from_str::<UploadFiles>(&format!("[{}]", items.join(","))).unwrap_err();
        assert!(error.to_string().contains("collection exceeds"));
        assert!(!error.to_string().contains("do_not_decode_this_path"));
    }

    #[test]
    fn upload_paths_reuse_the_workspace_utf8_byte_boundary_not_character_count() {
        for unit in ["x", "한"] {
            let fitting = unit.repeat(RelPath::MAX_BYTES / unit.len());
            assert!(UploadPath::try_from(fitting.clone()).is_ok());
            assert!(
                serde_json::from_str::<UploadPath>(&serde_json::to_string(&fitting).unwrap())
                    .is_ok()
            );
            let oversized = format!("{fitting}{unit}");
            assert!(matches!(
                UploadPath::try_from(oversized.clone()),
                Err(InvalidUpload::Path)
            ));
            assert!(
                serde_json::from_str::<UploadPath>(&serde_json::to_string(&oversized).unwrap())
                    .is_err()
            );
        }
    }
}
