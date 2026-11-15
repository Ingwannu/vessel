//! Bounded opaque TOML source and content revisions for conflict-aware editing.
//! Syntax interpretation belongs to vessel-workspace, never the wire crate.

use crate::{ProjectId, workspace::ContentHash};
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};

/// Canonical vessel.toml byte budget for both the typed editor and file parser.
/// The parser preserves its public `MAX_BYTES` name by re-exporting this value.
pub const MAX_CONFIG_BYTES: usize = 64 * 1024;

/// File text is private because a malformed request must not bypass its byte cap.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
#[serde(try_from = "String", into = "String")]
pub struct ConfigText(Arc<str>);

/// A source rejection never includes the possibly secret-bearing source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfigTextError {
    /// The byte cap is checked at decode as well as at filesystem read.
    #[error("project configuration exceeds {MAX_CONFIG_BYTES} bytes")]
    TooLarge,
    /// NUL cannot be part of a project TOML document.
    #[error("project configuration contains NUL")]
    Nul,
}

impl ConfigText {
    /// Validate before retaining text received from a client or disk.
    ///
    /// # Errors
    /// Refuses NUL and sources larger than `MAX_CONFIG_BYTES`.
    pub fn parse(text: &str) -> Result<Self, ConfigTextError> {
        if text.len() > MAX_CONFIG_BYTES {
            return Err(ConfigTextError::TooLarge);
        }
        if text.contains('\0') {
            return Err(ConfigTextError::Nul);
        }
        Ok(Self(Arc::from(text)))
    }

    /// The editor/validator explicitly asks for source; Debug never reveals it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ConfigText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConfigText")
            .field("bytes", &self.0.len())
            .finish()
    }
}
impl TryFrom<String> for ConfigText {
    type Error = ConfigTextError;
    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}
impl From<ConfigText> for String {
    fn from(text: ConfigText) -> Self {
        text.as_str().to_owned()
    }
}

/// Absence is distinct from an empty file, so concurrent file creation conflicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConfigRevision {
    /// There was no configuration file at the last read.
    Missing,
    /// A content hash permits external editors without relying on mtime resolution.
    Present {
        /// Hash only; configuration bytes never enter diagnostics.
        hash: ContentHash,
    },
}

/// Invalid TOML remains editable; its contents are not interpolated into errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ConfigValidity {
    /// Existing schema validator accepted the text.
    Valid,
    /// Reload/edit is permitted, but saving must pass the existing validator.
    Invalid,
}

/// Authoritative config snapshot, including missing files as an editable document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ConfigDocument {
    /// Source preserves comments and unrecognized keys across editing.
    pub text: ConfigText,
    /// Save precondition; never inferred by a client from source length.
    pub revision: ConfigRevision,
    /// Read does not fail merely because the source needs repair.
    pub validity: ConfigValidity,
}

/// Read only the exact active registered project's vessel.toml.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadProjectConfig {
    /// Registered identity, not an arbitrary filesystem root supplied by the client.
    pub project: ProjectId,
}

/// Saving changes source only; approval/execution are separate explicit actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WriteProjectConfig {
    /// The exact project whose registered root is rechecked before IO.
    pub project: ProjectId,
    /// A stale source is refused, never silently overwritten.
    pub expected: ConfigRevision,
    /// Bounded source preserves unknown TOML fields and comments.
    pub text: ConfigText,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_bounds_and_redaction_hold_at_decode() {
        assert!(ConfigText::parse(&"x".repeat(MAX_CONFIG_BYTES)).is_ok());
        assert_eq!(
            ConfigText::parse(&"x".repeat(MAX_CONFIG_BYTES.saturating_add(1))),
            Err(ConfigTextError::TooLarge)
        );
        assert_eq!(ConfigText::parse("x\0y"), Err(ConfigTextError::Nul));
        assert!(serde_json::from_str::<ConfigText>(r#""\u0000""#).is_err());
        let secret = "credential = 'private-value'";
        let text = ConfigText::parse(secret).unwrap();
        assert!(!format!("{text:?}").contains(secret));
        assert_eq!(
            serde_json::from_str::<ConfigText>(&serde_json::to_string(&text).unwrap()).unwrap(),
            text
        );
    }
    #[test]
    fn missing_and_present_empty_file_are_different_revisions() {
        let hash = "e".repeat(64).parse().unwrap();
        assert_ne!(ConfigRevision::Missing, ConfigRevision::Present { hash });
    }
}
