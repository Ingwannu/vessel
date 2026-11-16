//! Keys are opaque Unicode text, fingerprints/revisions have distinct units.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::CreationError;

/// Caller idempotency key, bounded without interpreting it as a filesystem path.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
#[cfg_attr(feature = "ts", ts(rename = "CreationKey"))]
pub struct Key(String);

impl Key {
    /// Paseo's create-key bound, in Unicode scalars rather than UTF-16 units.
    pub const MAX_CHARS: usize = 512;

    /// Parse at the external boundary; leading/trailing spaces retain identity.
    /// # Errors
    /// Refuses empty, control-containing or overlong keys without echoing them.
    pub fn parse(text: impl Into<String>) -> Result<Self, CreationError> {
        let text = text.into();
        if text.is_empty() {
            return Err(CreationError::EmptyKey);
        }
        if text.chars().count() > Self::MAX_CHARS {
            return Err(CreationError::KeyTooLong);
        }
        if text.chars().any(char::is_control) {
            return Err(CreationError::KeyControl);
        }
        Ok(Self(text))
    }

    /// Hash at the storage edge; never use a caller key as a file name or log field.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Key {
    type Error = CreationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}
impl From<Key> for String {
    fn from(value: Key) -> Self {
        value.0
    }
}
impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Key([redacted])")
    }
}

/// SHA-256 of normalized intent, distinct from a raw request or idempotency key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "CreationFingerprint"))]
pub struct Fingerprint(crate::BlobHash);

impl Fingerprint {
    /// Hashing belongs at the typed request/storage boundary, never in UI code.
    #[must_use]
    pub const fn new(hash: crate::BlobHash) -> Self {
        Self(hash)
    }
}

/// Ordered publication version, advanced only by a committed receipt transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "CreationRevision"))]
pub struct Revision(u64);

impl Revision {
    /// A newly admitted receipt has not published any completed side effect.
    pub const INITIAL: Self = Self(0);

    /// A revision never wraps, even after an artificially exhausted fixture.
    /// # Errors
    /// The counter cannot advance without reusing an earlier revision.
    pub const fn next(self) -> Result<Self, CreationError> {
        match self.0.checked_add(1) {
            Some(next) => Ok(Self(next)),
            None => Err(CreationError::RevisionExhausted),
        }
    }

    /// Compare transport/store observations without exposing a mutable counter.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}
