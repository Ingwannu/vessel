//! A composer's unsent text, kept by the bridge so it outlives the tab and
//! the launch (ADR 0028): [`DraftKey`] says whose composer, [`DraftText`]
//! the text, [`DraftsState`] every draft and where they are kept.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::SessionId;

use super::Kept;

/// Whose composer a draft is: the new-session page's (there is one), or a
/// session's. A session id is minted by its node and unique across
/// machines, so a renamed machine keeps its drafts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DraftKey {
    /// The new-session page's composer.
    New,
    /// A session's composer.
    Session {
        /// The session.
        session: SessionId,
    },
}

/// The longest [`DraftText`], in bytes of UTF-8 (agent-orchestrator's
/// limit for a message).
pub const MAX_DRAFT: usize = 128 * 1024;

/// A draft's text: at most [`MAX_DRAFT`] bytes of UTF-8; the empty text
/// removes the draft. `Debug` shows its length only: a draft is whatever
/// a person typed, and has no place in a log line or a test's failure.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "String", into = "String")]
pub struct DraftText(String);

/// Text longer than [`MAX_DRAFT`] bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a draft is at most {MAX_DRAFT} bytes of UTF-8")]
pub struct DraftTooLong;

impl DraftText {
    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether it is empty (which removes the draft).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl TryFrom<String> for DraftText {
    type Error = DraftTooLong;

    fn try_from(text: String) -> Result<Self, DraftTooLong> {
        if text.len() <= MAX_DRAFT {
            Ok(Self(text))
        } else {
            Err(DraftTooLong)
        }
    }
}

impl From<DraftText> for String {
    fn from(text: DraftText) -> Self {
        text.0
    }
}

impl fmt::Debug for DraftText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DraftText({} bytes)", self.0.len())
    }
}

/// One draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Draft {
    /// Whose composer.
    pub key: DraftKey,
    /// The text (never empty: an empty save removes the draft).
    pub text: DraftText,
    /// When it was last saved.
    pub at: DateTime<Utc>,
}

/// Every draft, and where they are kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DraftsState {
    /// Where they are kept.
    pub kept: Kept,
    /// The drafts, ordered by key (the new-session page's first).
    pub drafts: Vec<Draft>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draft_is_bounded_and_never_printed() {
        let text: DraftText = serde_json::from_str("\"my secret plan\"").unwrap();
        assert_eq!(format!("{text:?}"), "DraftText(14 bytes)");
        // `é` is two bytes: 65 536 of them are the bound exactly.
        let at_bound = format!("\"{}\"", "é".repeat(65_536));
        assert!(serde_json::from_str::<DraftText>(&at_bound).is_ok());
        let past = format!("\"{}x\"", "é".repeat(65_536));
        assert!(serde_json::from_str::<DraftText>(&past).is_err());
        let key: DraftKey = serde_json::from_str(
            r#"{"kind":"session","session":"0d4b6a35-37d2-4d5c-9f3e-6f1a2b7c8d90"}"#,
        )
        .unwrap();
        assert!(matches!(key, DraftKey::Session { .. }));
        assert_eq!(
            serde_json::to_string(&DraftKey::New).unwrap(),
            r#"{"kind":"new"}"#
        );
    }
}
