//! Plugin settings documents (ADR 0115, "Settings", 2026-11-20): what a
//! helper declares, what the node asks it, and the three Admin methods.
//!
//! [Decision Log]
//! - Purpose: Paseo's plugins keep host-scoped settings documents
//!   (`defineSettings`, packages/plugin/src/settings.ts) with a schema
//!   version, migration and an optimistic revision; clients read, save and
//!   reset them through the plugin (`settings.<id>.read|write|reset`).
//! - Constraint: Vessel's helper may be any program, so it cannot be relied
//!   on to write files atomically and owner-only, and its file access
//!   should stay narrow (coordinator ruling 2026-11-20).
//! - Choice: the node owns the store (the envelope, the revision, the
//!   conflict check, the atomic owner-only write) and the helper owns the
//!   schema: the node asks it to parse a document (defaults applied) or to
//!   migrate one from an older version, and tells it each document's state
//!   (`changed`), which is Paseo's server-side `read()` and `subscribe()`.
//!   A stated difference from Paseo: the helper never writes its own
//!   settings files.
//! - A refusal is a verdict, not a protocol error, so the helper's words
//!   can reach the person (Paseo shows its schema's messages): bounded,
//!   without controls, to Admin callers only ([`SettingsMessage`]). This
//!   is ADR 0079's stated exception to safe error classes.
//! - Shapes follow Paseo: read is `ready` or `invalid`, write and reset
//!   `saved`, `conflict` or `invalid`; ids match `^[a-z][a-z0-9_-]*$`;
//!   versions are positive safe integers.

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{HelperPayload, PluginId, SettingsId};
use crate::RawJson;

/// Settings documents one installation may declare.
pub const MAX_SETTINGS_DOCUMENTS: usize = 16;

/// The longest refusal text a caller sees, in bytes.
pub const MAX_SETTINGS_MESSAGE_BYTES: usize = 1024;

/// JavaScript's largest safe integer (Paseo's `Number.isSafeInteger`).
const MAX_SETTINGS_VERSION: u64 = (1 << 53) - 1;

/// The revision of a document that has never been saved (Paseo's).
const MISSING: &str = "missing";

/// A settings document's schema version, independent of its revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(try_from = "u64", into = "u64")]
pub struct SettingsVersion(u64);

impl SettingsVersion {
    /// `version` when it is from 1 to 2^53 - 1.
    #[must_use]
    pub const fn new(version: u64) -> Option<Self> {
        if version >= 1 && version <= MAX_SETTINGS_VERSION {
            Some(Self(version))
        } else {
            None
        }
    }

    /// The number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Why a declaration or a value cannot be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SettingsValueError {
    /// Zero, or past JavaScript's safe integers.
    #[error("a settings version is a whole number from 1 to 2^53 - 1")]
    Version,
    /// Neither `missing` nor a sha256 in lowercase hex.
    #[error("a settings revision is `missing` or 64 lowercase hex characters")]
    Revision,
    /// More than [`MAX_SETTINGS_DOCUMENTS`] documents.
    #[error("an installation declares at most {MAX_SETTINGS_DOCUMENTS} settings documents")]
    TooMany,
    /// Two declarations with one id.
    #[error("a settings id is declared twice")]
    Duplicate,
    /// Longer than [`MAX_SETTINGS_MESSAGE_BYTES`] or holding controls.
    #[error(
        "a settings message is at most {MAX_SETTINGS_MESSAGE_BYTES} bytes without control characters"
    )]
    Message,
}

impl TryFrom<u64> for SettingsVersion {
    type Error = SettingsValueError;
    fn try_from(version: u64) -> Result<Self, Self::Error> {
        Self::new(version).ok_or(SettingsValueError::Version)
    }
}

impl From<SettingsVersion> for u64 {
    fn from(version: SettingsVersion) -> Self {
        version.0
    }
}

/// An opaque revision: the sha256 of the stored bytes, or `missing`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
#[serde(try_from = "String", into = "String")]
pub struct SettingsRevision(Arc<str>);

impl SettingsRevision {
    /// A document that was never saved.
    #[must_use]
    pub fn missing() -> Self {
        Self(Arc::from(MISSING))
    }

    /// A sha256 digest in lowercase hex; `None` otherwise.
    #[must_use]
    pub fn from_sha256_hex(hex: &str) -> Option<Self> {
        (hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
        .then(|| Self(Arc::from(hex)))
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SettingsRevision {
    type Error = SettingsValueError;
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        if raw == MISSING {
            return Ok(Self::missing());
        }
        Self::from_sha256_hex(&raw).ok_or(SettingsValueError::Revision)
    }
}

impl From<SettingsRevision> for String {
    fn from(revision: SettingsRevision) -> Self {
        revision.0.as_ref().to_owned()
    }
}

/// Refusal text a person reads: at most [`MAX_SETTINGS_MESSAGE_BYTES`],
/// no control characters.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
#[serde(try_from = "String", into = "String")]
pub struct SettingsMessage(Arc<str>);

impl SettingsMessage {
    /// `raw` with its control characters turned into spaces and cut at a
    /// character boundary to fit; the node's way to pass a helper's words.
    #[must_use]
    pub fn bounded(raw: &str) -> Self {
        let mut text = String::new();
        for c in raw.chars() {
            let c = if c.is_control() { ' ' } else { c };
            if text.len().saturating_add(c.len_utf8()) > MAX_SETTINGS_MESSAGE_BYTES {
                break;
            }
            text.push(c);
        }
        Self(Arc::from(text.trim()))
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SettingsMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SettingsMessage")
            .field(&self.0.len())
            .finish()
    }
}

impl TryFrom<String> for SettingsMessage {
    type Error = SettingsValueError;
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        if raw.len() > MAX_SETTINGS_MESSAGE_BYTES || raw.chars().any(char::is_control) {
            return Err(SettingsValueError::Message);
        }
        Ok(Self(Arc::from(raw)))
    }
}

impl From<SettingsMessage> for String {
    fn from(message: SettingsMessage) -> Self {
        message.0.as_ref().to_owned()
    }
}

/// One document a helper declares in `ready` (Paseo's `registerSettings`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsDeclaration {
    /// Its id, also its file's name.
    pub id: SettingsId,
    /// The schema version the helper parses.
    pub version: SettingsVersion,
}

/// A helper's settings documents: at most [`MAX_SETTINGS_DOCUMENTS`], ids
/// unique.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(
    try_from = "Vec<SettingsDeclaration>",
    into = "Vec<SettingsDeclaration>"
)]
pub struct SettingsDeclarations(Vec<SettingsDeclaration>);

impl SettingsDeclarations {
    /// `declarations` when they fit.
    ///
    /// # Errors
    /// Too many, or an id twice.
    pub fn new(declarations: Vec<SettingsDeclaration>) -> Result<Self, SettingsValueError> {
        if declarations.len() > MAX_SETTINGS_DOCUMENTS {
            return Err(SettingsValueError::TooMany);
        }
        for (at, declaration) in declarations.iter().enumerate() {
            if declarations
                .iter()
                .skip(at.saturating_add(1))
                .any(|other| other.id == declaration.id)
            {
                return Err(SettingsValueError::Duplicate);
            }
        }
        Ok(Self(declarations))
    }

    /// The version `id` was declared at, if it was.
    #[must_use]
    pub fn version_of(&self, id: &SettingsId) -> Option<SettingsVersion> {
        self.0
            .iter()
            .find(|declaration| &declaration.id == id)
            .map(|declaration| declaration.version)
    }

    /// The declarations in order.
    pub fn iter(&self) -> impl Iterator<Item = &SettingsDeclaration> {
        self.0.iter()
    }
}

impl TryFrom<Vec<SettingsDeclaration>> for SettingsDeclarations {
    type Error = SettingsValueError;
    fn try_from(declarations: Vec<SettingsDeclaration>) -> Result<Self, Self::Error> {
        Self::new(declarations)
    }
}

impl From<SettingsDeclarations> for Vec<SettingsDeclaration> {
    fn from(declarations: SettingsDeclarations) -> Self {
        declarations.0
    }
}

/// What the node asks a helper about one document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case", deny_unknown_fields)]
pub enum SettingsStep {
    /// Apply the schema's defaults and validate; answer a [`SettingsVerdict`].
    Parse {
        /// The document's values (`{}` for a missing one or a reset).
        values: HelperPayload,
    },
    /// Turn values stored at `from` into the current version; answer a
    /// [`SettingsVerdict`].
    Migrate {
        /// The stored version, older than the declared one.
        from: SettingsVersion,
        /// The stored values.
        values: HelperPayload,
    },
    /// The document's state now; answered with any value, which is not
    /// read.
    Changed {
        /// Ready with its values, or invalid.
        state: SettingsNotice,
    },
}

impl SettingsStep {
    /// The payload the actor's byte budget counts.
    #[must_use]
    pub const fn payload(&self) -> Option<&HelperPayload> {
        match self {
            Self::Parse { values }
            | Self::Migrate { values, .. }
            | Self::Changed {
                state: SettingsNotice::Ready { values, .. },
            } => Some(values),
            Self::Changed {
                state: SettingsNotice::Invalid { .. },
            } => None,
        }
    }
}

/// A document's state as the node tells its helper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum SettingsNotice {
    /// Parsed values at a revision.
    Ready {
        /// The stored revision.
        revision: SettingsRevision,
        /// The parsed values.
        values: HelperPayload,
    },
    /// The stored document does not parse; it is kept as it is.
    Invalid {
        /// The stored revision.
        revision: SettingsRevision,
    },
}

/// A helper's answer to `parse` and `migrate`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case", deny_unknown_fields)]
pub enum SettingsVerdict {
    /// The document, complete, at the declared version.
    Valid {
        /// Its values.
        values: RawJson,
    },
    /// Why not, in words for the person.
    Invalid {
        /// Any text; the node bounds it before a caller sees it.
        message: String,
    },
}

/// `plugin.settings.read`: one document's state. Needs `admin`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ReadPluginSettings {
    /// The installation.
    pub plugin: PluginId,
    /// The document.
    pub settings: SettingsId,
}

/// `plugin.settings.write`: save a complete document. Needs `admin`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct WritePluginSettings {
    /// The installation.
    pub plugin: PluginId,
    /// The document.
    pub settings: SettingsId,
    /// The revision the caller read; another one is a conflict.
    pub revision: SettingsRevision,
    /// The complete document.
    pub values: RawJson,
}

/// `plugin.settings.reset`: replace a document with its defaults. Needs
/// `admin`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ResetPluginSettings {
    /// The installation.
    pub plugin: PluginId,
    /// The document.
    pub settings: SettingsId,
    /// The revision the caller read; another one is a conflict.
    pub revision: SettingsRevision,
}

/// Reply to `plugin.settings.read` (Paseo's `SettingsReadResultSchema`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PluginSettingsRead {
    /// The parsed document.
    Ready {
        /// What a save must name.
        revision: SettingsRevision,
        /// The values, defaults applied.
        values: RawJson,
    },
    /// The stored document does not parse; it is kept until a save or a
    /// reset replaces it.
    Invalid {
        /// What a reset must name.
        revision: SettingsRevision,
        /// Why, in words for the person.
        error: SettingsMessage,
    },
}

/// Reply to `plugin.settings.write` and `plugin.settings.reset` (Paseo's
/// `SettingsWriteResultSchema`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PluginSettingsWrite {
    /// Saved: the new revision and the values as parsed.
    Saved {
        /// The new revision.
        revision: SettingsRevision,
        /// The values, defaults applied.
        values: RawJson,
    },
    /// The document changed since the caller read it.
    Conflict {
        /// Paseo's words: read again before saving.
        error: SettingsMessage,
    },
    /// The values or the stored version were refused.
    Invalid {
        /// Why, in words for the person.
        error: SettingsMessage,
    },
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
