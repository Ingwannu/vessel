//! Upload names are transported without authorizing a host filesystem read.
//!
//! The engine must resolve/stage regular files under the authenticated session's
//! workspace. Parsing a path here neither checks existence nor follows links.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::arguments::ElementArgs;

/// Upload input refusals do not reflect a path that may contain private data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidUpload {
    /// Empty path lists cannot represent an upload intent.
    #[error("browser upload requires at least one file")]
    EmptyList,
    /// An empty or NUL-containing path cannot name a regular file.
    #[error("upload path must be nonempty and contain no NUL")]
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
        if value.is_empty() || value.contains('\0') {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<UploadPath>", into = "Vec<UploadPath>")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct UploadFiles(Vec<UploadPath>);

impl UploadFiles {
    /// Borrow every file before performing any stage/attach side effect.
    #[must_use]
    pub fn as_slice(&self) -> &[UploadPath] {
        &self.0
    }
}

impl TryFrom<Vec<UploadPath>> for UploadFiles {
    type Error = InvalidUpload;
    fn try_from(value: Vec<UploadPath>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(InvalidUpload::EmptyList)
        } else {
            Ok(Self(value))
        }
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
}
