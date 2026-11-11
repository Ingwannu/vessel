//! A session's title (protocol 9, ADR 0022): parsed once, valid wherever
//! it is held.
//!
//! [Decision Log]
//! - Purpose: a board of cards named after folders is not readable, so
//!   every session gets a name a person recognizes: one they typed
//!   (`SessionSpec.title`, `session.rename`) or the first line of its
//!   first prompt.
//! - Constraint: a title is shown on one line of a card, a tab and a
//!   terminal; a newline or a control character in one breaks all three,
//!   and a title of a whole pasted file fills them. AGENTS.md section 2:
//!   a value with a shape is a type whose constructor checks it.
//! - Alternatives: a `String` checked where the node receives it (every
//!   other holder, the store and every client, would have to trust or
//!   repeat the check; a stored row could hold what the check refuses).
//! - Choice: [`SessionTitle`], built only by [`FromStr`] (a typed title:
//!   trimmed, not empty, at most [`SessionTitle::MAX_CHARS`] characters,
//!   no control characters; anything else is refused, not cut, so a
//!   client never shows another title than the one it sent) and by
//!   [`SessionTitle::from_prompt`] (the first non-empty line, its control
//!   characters made spaces, cut at [`SessionTitle::FROM_PROMPT_CHARS`]:
//!   a prompt was never meant as a title, so it is cut rather than
//!   refused). Decoding goes through the parse, so a bad title on the wire
//!   or in the store is a decode error, never a value.
//! - Characters, not bytes: a Korean title is cut where a person counts.
//!   `char`s, not grapheme clusters, so no dependency; a cut may split a
//!   combining sequence, which only the last character of a long line
//!   risks.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// A session's title: one line of text a person reads.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct SessionTitle(Arc<str>);

/// Why a typed title was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidTitle {
    /// Nothing but white space.
    Empty,
    /// A newline, a tab or another control character.
    ControlCharacter,
    /// More than [`SessionTitle::MAX_CHARS`] characters (after trimming).
    TooLong {
        /// How many it has.
        chars: usize,
    },
}

impl fmt::Display for InvalidTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("a title cannot be empty"),
            Self::ControlCharacter => {
                f.write_str("a title is one line, without tabs or other control characters")
            }
            Self::TooLong { chars } => write!(
                f,
                "a title is at most {} characters, not {chars}",
                SessionTitle::MAX_CHARS
            ),
        }
    }
}

impl std::error::Error for InvalidTitle {}

impl SessionTitle {
    /// The longest title a person may give, in characters.
    pub const MAX_CHARS: usize = 120;

    /// Where a title taken from a prompt is cut, in characters.
    pub const FROM_PROMPT_CHARS: usize = 80;

    /// The title a prompt gives an untitled session: its first non-empty
    /// line, trimmed, control characters (tabs) as spaces, cut at
    /// [`FROM_PROMPT_CHARS`](Self::FROM_PROMPT_CHARS) characters. `None`
    /// when the text has no such line.
    #[must_use]
    pub fn from_prompt(text: &str) -> Option<Self> {
        let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
        let cut: String = line
            .chars()
            .map(|character| {
                if character.is_control() {
                    ' '
                } else {
                    character
                }
            })
            .take(Self::FROM_PROMPT_CHARS)
            .collect();
        let title = cut.trim_end();
        (!title.is_empty()).then(|| Self(Arc::from(title)))
    }

    /// The title.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for SessionTitle {
    type Err = InvalidTitle;

    /// A typed title, trimmed; refused rather than changed otherwise.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let title = text.trim();
        if title.is_empty() {
            return Err(InvalidTitle::Empty);
        }
        if title.chars().any(char::is_control) {
            return Err(InvalidTitle::ControlCharacter);
        }
        let chars = title.chars().count();
        if chars > Self::MAX_CHARS {
            return Err(InvalidTitle::TooLong { chars });
        }
        Ok(Self(Arc::from(title)))
    }
}

impl fmt::Display for SessionTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

crate::key::serde_via_text!(SessionTitle);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_title_is_one_trimmed_line() {
        let title: SessionTitle = "  Fix the build ".parse().unwrap();
        assert_eq!(title.as_str(), "Fix the build");
        assert_eq!(" \t ".parse::<SessionTitle>(), Err(InvalidTitle::Empty));
        assert_eq!(
            "two\nlines".parse::<SessionTitle>(),
            Err(InvalidTitle::ControlCharacter)
        );
        assert_eq!(
            "a\tb".parse::<SessionTitle>(),
            Err(InvalidTitle::ControlCharacter)
        );
        let longest = "가".repeat(SessionTitle::MAX_CHARS);
        assert!(
            longest.parse::<SessionTitle>().is_ok(),
            "characters, not bytes"
        );
        assert_eq!(
            "a".repeat(121).parse::<SessionTitle>(),
            Err(InvalidTitle::TooLong { chars: 121 })
        );
    }

    #[test]
    fn a_bad_title_does_not_decode() {
        let title: SessionTitle = serde_json::from_str(r#""Release notes""#).unwrap();
        assert_eq!(serde_json::to_string(&title).unwrap(), r#""Release notes""#);
        assert!(serde_json::from_str::<SessionTitle>(r#""""#).is_err());
        assert!(serde_json::from_str::<SessionTitle>(r#""a\nb""#).is_err());
    }

    #[test]
    fn a_prompt_gives_its_first_line_cut_at_eighty() {
        let title = SessionTitle::from_prompt("\n\n  Fix the failing test\tin ci  \nthen push");
        assert_eq!(
            title.as_ref().map(SessionTitle::as_str),
            Some("Fix the failing test in ci")
        );
        assert_eq!(SessionTitle::from_prompt("  \n \r\n"), None);
        let long = SessionTitle::from_prompt(&"한".repeat(100)).unwrap();
        assert_eq!(
            long.as_str().chars().count(),
            SessionTitle::FROM_PROMPT_CHARS
        );
        // What a prompt gives is always a title a person could type.
        assert_eq!(long.as_str().parse::<SessionTitle>(), Ok(long));
    }
}
