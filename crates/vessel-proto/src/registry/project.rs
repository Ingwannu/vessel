//! Project metadata and plain directory creation; neither operation starts agents.

use crate::{
    ProjectId,
    workspace::{FolderName, InvalidText, ProjectName},
};
use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf};

/// One new folder's name, checked before any directory is created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
#[serde(try_from = "String", into = "String")]
pub struct DirectoryName(FolderName);

/// The source error is retained without echoing an arbitrary path.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DirectoryNameError {
    /// Leading/trailing spaces have ambiguous platform filesystem semantics.
    #[error("a directory name cannot have leading or trailing whitespace")]
    Whitespace,
    /// Existing portable folder validation is shared, not reimplemented.
    #[error("invalid directory name: {0}")]
    Invalid(#[source] InvalidText),
}
impl DirectoryName {
    /// Require one portable component with no surrounding whitespace.
    ///
    /// # Errors
    /// Rejects whitespace, traversal, absolute paths and invalid folder components.
    pub fn parse(text: &str) -> Result<Self, DirectoryNameError> {
        if text != text.trim() {
            return Err(DirectoryNameError::Whitespace);
        }
        text.parse().map(Self).map_err(DirectoryNameError::Invalid)
    }
    /// Used only to join the already-validated parent directory.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl fmt::Display for DirectoryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl TryFrom<String> for DirectoryName {
    type Error = DirectoryNameError;
    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}
impl From<DirectoryName> for String {
    fn from(name: DirectoryName) -> Self {
        name.as_str().to_owned()
    }
}

/// Create a single empty directory, register it, and do not initialize Git.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateProjectDirectory {
    /// Existing absolute parent on the selected node; never a client-local path.
    pub parent: PathBuf,
    /// One component, checked while decoding before filesystem mutation.
    pub name: DirectoryName,
}

/// Host-specific presentation remains separate from backward-compatible `ProjectInfo`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProjectMetadata {
    /// Identity never changes when its displayed name is edited.
    pub project: ProjectId,
    /// Folder-derived label for resetting an override.
    pub derived_name: ProjectName,
    /// None means use `derived_name`; a client never invents a local-only override.
    pub custom_name: Option<ProjectName>,
}

/// Load persisted presentation without reading any filesystem path from the client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadProjectMetadata {
    /// Host-local project id.
    pub project: ProjectId,
}

/// Rename presentation only, with an explicit reset represented by absence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RenameProject {
    /// Existing identity; files and directory names stay put.
    pub project: ProjectId,
    /// None resets the label to the recorded directory's name.
    pub custom_name: Option<ProjectName>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directory_components_are_portable_and_decode_cannot_bypass_validation() {
        for invalid in [
            "",
            ".",
            "..",
            "../escape",
            "/absolute",
            "C:\\escape",
            "a/b",
            " padded",
            "padded ",
            "a\0b",
        ] {
            assert!(
                DirectoryName::parse(invalid).is_err(),
                "accepted {invalid:?}"
            );
            assert!(
                serde_json::from_str::<DirectoryName>(&serde_json::to_string(invalid).unwrap())
                    .is_err()
            );
        }
        assert_eq!(
            DirectoryName::parse("새 프로젝트").unwrap().as_str(),
            "새 프로젝트"
        );
    }
}
