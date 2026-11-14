//! The completion time, parsed once.
//!
//! The prefs file is the link's. What crosses into the model is this
//! timestamp, not the JSON. An invalid value is the caller's to drop;
//! this type cannot hold one.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

/// When this client finished or skipped setup.
///
/// Stored as UTC. An offset in the text is accepted and converted; the
/// value does not keep the original spelling, so two writings of the
/// same instant compare equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CompletedAt(DateTime<Utc>);

/// Why text is not a [`CompletedAt`].
#[derive(Debug, thiserror::Error)]
#[error("a completion time is an RFC 3339 timestamp")]
pub struct BadTime {
    /// Chrono's parse failure, kept so the cause is not discarded.
    /// This error's own message does not repeat the text.
    #[source]
    source: chrono::ParseError,
}

impl CompletedAt {
    /// `text` as an RFC 3339 timestamp.
    ///
    /// # Errors
    /// [`BadTime`] when `text` is not RFC 3339. A date alone is not.
    pub fn parse(text: &str) -> Result<Self, BadTime> {
        DateTime::parse_from_rfc3339(text)
            .map(|parsed| Self(parsed.with_timezone(&Utc)))
            .map_err(|source| BadTime { source })
    }

    /// The instant, for a caller that draws it. The model never does.
    #[must_use]
    pub const fn get(self) -> DateTime<Utc> {
        self.0
    }
}

impl TryFrom<String> for CompletedAt {
    type Error = BadTime;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}

impl From<CompletedAt> for String {
    fn from(at: CompletedAt) -> Self {
        at.0.to_rfc3339_opts(SecondsFormat::Millis, true)
    }
}

/// What the model has been told about the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Loaded {
    /// [`crate::onboarding::Onboarding::note_completion`] has not been called.
    Waiting,
    /// No record on this client.
    Pending,
    /// A record was read. The instant stays with the caller; the model
    /// only needs to know that setup already finished.
    Completed,
    /// The prefs file could not be read. Treated as finished, and not written.
    Unreadable,
}

/// The JSON object the prefs file keeps under `setup`.
///
/// One field, the instant. Paseo's record is the same fact under another
/// spelling; this file is already snake case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupRecord {
    /// When setup was finished or skipped on this client.
    pub completed_at: CompletedAt,
}
