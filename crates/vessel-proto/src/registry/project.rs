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
        if text != crate::workspace::trim_project_text(text) {
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
    /// Explicit null or blank resets the label; omission is not a reset instruction.
    #[serde(deserialize_with = "decode_custom_name")]
    pub custom_name: Option<ProjectName>,
}

impl RenameProject {
    /// Normalize user text by the same JavaScript-edge rule as Paseo before validation.
    ///
    /// # Errors
    /// Refuses labels outside the existing portable ProjectName contract.
    pub fn from_text(project: ProjectId, text: Option<&str>) -> Result<Self, InvalidText> {
        Ok(Self {
            project,
            custom_name: custom_name(text)?,
        })
    }
    /// Validate programmatic Rust requests too, without relying only on serde normalization.
    ///
    /// # Errors
    /// The normalized label must satisfy ProjectName's existing bound.
    pub fn normalized_custom_name(&self) -> Result<Option<ProjectName>, InvalidText> {
        custom_name(self.custom_name.as_ref().map(ProjectName::as_str))
    }
}

fn custom_name(text: Option<&str>) -> Result<Option<ProjectName>, InvalidText> {
    match text.map(crate::workspace::trim_project_text) {
        None | Some("") => Ok(None),
        Some(name) => name.parse().map(Some),
    }
}

fn decode_custom_name<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<ProjectName>, D::Error> {
    let text = Option::<String>::deserialize(decoder)?;
    custom_name(text.as_deref()).map_err(serde::de::Error::custom)
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
            "\u{feff}hidden",
            "hidden\u{feff}",
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

    #[test]
    fn rename_requires_explicit_reset_and_normalizes_javascript_whitespace_before_the_name_bound() {
        let project = ProjectId::random();
        assert!(
            serde_json::from_value::<RenameProject>(serde_json::json!({"project":project}))
                .is_err()
        );
        for name in [None, Some(""), Some(" \t\u{feff}\u{2028}")] {
            let request = RenameProject::from_text(project, name).unwrap();
            assert_eq!(request.custom_name, None);
            let wire = serde_json::json!({"project":project,"custom_name":name});
            assert_eq!(
                serde_json::from_value::<RenameProject>(wire).unwrap(),
                request
            );
        }
        let name = "x".repeat(80);
        let padded = format!("\u{feff}{name}\u{2029}");
        let request = serde_json::from_value::<RenameProject>(
            serde_json::json!({"project":project,"custom_name":padded}),
        )
        .unwrap();
        assert_eq!(request.custom_name.unwrap().as_str(), name);
        assert!(RenameProject::from_text(project, Some(&"x".repeat(81))).is_err());
    }
}
