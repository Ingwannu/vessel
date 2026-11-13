//! Searching every session's messages (protocol 12, milestone 11's track
//! A2): `session.search` and what it answers.
//!
//! The node's log holds records, not messages; the store's index makes a
//! message of a prompt or of a run of the agent's reply chunks (ADR 0037),
//! and a hit names the record where it starts, so a client opens the
//! session there. The plan's "Track A2: contract" is the reader's summary.
//!
//! [Decision Log]
//! - Purpose: find what was said in any session, kept or ended, without a
//!   client reading every log (a node keeps up to 500 ended sessions of up
//!   to 64 MiB each).
//! - Alternatives: a client-side search over the logs it has read
//!   (Paseo's in-session find; it cannot see a session never opened); the
//!   references' palette-sized answers with no way past the first page
//!   (agent-orchestrator returns 8 hits).
//! - Choice: a read call answered from the node's index, newest first,
//!   paged by a cursor the node mints, each page bounded in the work it
//!   does as well as in what it returns.
//! - Snippets are typed parts (`matched` or not), not marker characters in
//!   a string (agent-orchestrator wraps matches in `\x02`/`\x03`): a client
//!   never parses text to find what to highlight.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::id::{EventSeq, SessionId};

/// Why a search's value was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidSearch {
    /// Which value.
    pub what: &'static str,
    /// The rule it breaks, for people.
    pub rule: &'static str,
}

impl fmt::Display for InvalidSearch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} must be {}", self.what, self.rule)
    }
}

impl std::error::Error for InvalidSearch {}

/// What a person searches for: 1 to [`SearchQuery::MAX_CHARS`] characters
/// once trimmed, at most [`SearchQuery::MAX_WORDS`] words. Every word must
/// be in a message for it to match.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct SearchQuery(Arc<str>);

impl SearchQuery {
    /// The longest query, in characters.
    pub const MAX_CHARS: usize = 256;
    /// The most words a query may have.
    pub const MAX_WORDS: usize = 16;

    /// The query, trimmed.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Its words, split on white space, in the order typed.
    pub fn words(&self) -> impl Iterator<Item = &str> {
        self.0.split_whitespace()
    }
}

impl FromStr for SearchQuery {
    type Err = InvalidSearch;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let trimmed = text.trim();
        let chars = trimmed.chars().count();
        let words = trimmed.split_whitespace().count();
        if (1..=Self::MAX_CHARS).contains(&chars)
            && words <= Self::MAX_WORDS
            && !trimmed.chars().any(char::is_control)
        {
            Ok(Self(Arc::from(trimmed)))
        } else {
            Err(InvalidSearch {
                what: "a search",
                rule: "1 to 256 characters once trimmed, at most 16 words, \
                       without control characters",
            })
        }
    }
}

impl fmt::Display for SearchQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

crate::key::serde_via_text!(SearchQuery);

/// How many hits a page may hold: 1 to [`SearchLimit::MAX`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct SearchLimit(u8);

impl SearchLimit {
    /// The most hits a page holds.
    pub const MAX: u8 = 100;
    /// A page's hits when the request names none.
    pub const DEFAULT: Self = Self(20);

    /// A limit, if within `1..=MAX`.
    #[must_use]
    pub const fn new(value: u8) -> Option<Self> {
        if value >= 1 && value <= Self::MAX {
            Some(Self(value))
        } else {
            None
        }
    }

    /// The limit.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl<'de> Deserialize<'de> for SearchLimit {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u8::deserialize(deserializer)?;
        Self::new(value).ok_or_else(|| serde::de::Error::custom("a search limit must be 1 to 100"))
    }
}

/// Where the next page of a search starts: the node's handle, handed back
/// as it came (a decimal number in a string, so a page's JavaScript never
/// rounds it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct SearchCursor(i64);

impl SearchCursor {
    /// Wrap the store's position (for the store and tests).
    #[must_use]
    pub const fn new(position: i64) -> Self {
        Self(position)
    }

    /// The store's position.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl FromStr for SearchCursor {
    type Err = InvalidSearch;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse::<i64>()
            .ok()
            .filter(|position| *position >= 0)
            .map(Self)
            .ok_or(InvalidSearch {
                what: "a search cursor",
                rule: "the `next` of an earlier page",
            })
    }
}

impl fmt::Display for SearchCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

crate::key::serde_via_text!(SearchCursor);

/// Who said a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// A person's (or a starter's) prompt, a steer, or a replayed prompt.
    Prompt,
    /// The agent's visible reply.
    Reply,
}

/// One piece of a snippet: text, and whether it is a match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SnippetPart {
    /// The text.
    pub text: String,
    /// Whether the text is one of the query's words.
    pub matched: bool,
}

/// A window of a message around its first match: at most
/// [`Snippet::MAX_CHARS`] characters, white space runs as one space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Snippet {
    /// The window, in order.
    pub parts: Vec<SnippetPart>,
    /// The message goes on before the window.
    pub cut_before: bool,
    /// The message goes on after the window.
    pub cut_after: bool,
}

impl Snippet {
    /// The most characters a snippet shows.
    pub const MAX_CHARS: usize = 240;
}

/// A message that matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SearchHit {
    /// The session it is in.
    pub session: SessionId,
    /// The record where the message (or the piece of a long one) starts.
    pub seq: EventSeq,
    /// When that record was written.
    pub at: DateTime<Utc>,
    /// Who said it.
    pub role: MessageRole,
    /// Where the words are in it.
    pub snippet: Snippet,
}

/// `session.search`: every session's messages (or one session's), newest
/// first. Needs `read` (protocol 12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SearchMessages {
    /// The words.
    pub query: SearchQuery,
    /// Only this session's messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionId>,
    /// Continue from an earlier page's `next`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<SearchCursor>,
    /// At most this many hits ([`SearchLimit::DEFAULT`] when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<SearchLimit>,
}

/// One page of a search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SearchPage {
    /// The hits, newest first.
    pub hits: Vec<SearchHit>,
    /// Present while older messages may still match: hand it back as
    /// `before`. A page can hold fewer hits than asked and still have it,
    /// when the work a page may do ran out first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<SearchCursor>,
    /// With `next`: the time of the oldest message this page read, so every
    /// match newer than it is in this page or an earlier one. A client
    /// that merges several machines' pages by time shows a machine's hits
    /// only down to its `reached` until it asks for the next page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reached: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_is_trimmed_and_bounded() {
        let query: SearchQuery = "  세션 검색  ".parse().unwrap();
        assert_eq!(query.as_str(), "세션 검색");
        assert_eq!(query.words().collect::<Vec<_>>(), ["세션", "검색"]);
        assert!("   ".parse::<SearchQuery>().is_err());
        assert!("a\u{7}b".parse::<SearchQuery>().is_err());
        assert!("가".repeat(256).parse::<SearchQuery>().is_ok());
        assert!("가".repeat(257).parse::<SearchQuery>().is_err());
        let seventeen = vec!["w"; 17].join(" ");
        assert!(seventeen.parse::<SearchQuery>().is_err());
    }

    #[test]
    fn a_limit_is_one_to_a_hundred_on_the_wire() {
        assert!(serde_json::from_str::<SearchLimit>("0").is_err());
        assert!(serde_json::from_str::<SearchLimit>("101").is_err());
        assert_eq!(
            serde_json::from_str::<SearchLimit>("100").unwrap().get(),
            100
        );
    }

    #[test]
    fn a_cursor_is_a_string_of_a_position() {
        let cursor = SearchCursor::new(1_789_000_000_000_000_123);
        let text = serde_json::to_string(&cursor).unwrap();
        assert_eq!(text, "\"1789000000000000123\"");
        assert_eq!(serde_json::from_str::<SearchCursor>(&text).unwrap(), cursor);
        assert!(serde_json::from_str::<SearchCursor>("\"-1\"").is_err());
        assert!(serde_json::from_str::<SearchCursor>("12").is_err());
    }
}
