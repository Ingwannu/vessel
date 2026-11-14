//! Why a TOML file of the client's (the node book, a project file, a
//! device credential) did not read: where, and what kind of problem, in
//! the client's own words. Never the file's text.
//!
//! [Decision Log]
//! - Purpose: a book whose `token = "…"` line held the token itself
//!   (the book takes `token = { file = "…" }`) showed that line, token
//!   and all, in the app's footer and the CLI's error, because the error
//!   carried `toml_edit`'s message, which quotes the offending line and
//!   often the value (`invalid type: string "…"`). A secret on screen is
//!   shown to whoever looks (the coordinator's review of 0.0.0.0.8,
//!   2026-11-13).
//! - Constraint: the parser's message is the only description it gives,
//!   and any part of it may quote the file.
//! - Choice: keep only the position (line and column, from the error's
//!   span) and a kind of problem read from the message's fixed serde
//!   wording; never keep the message. A key is named only when it is one
//!   of the file's own keys (an allow list the caller passes), so even a
//!   key is never echoed from the file.
//! - Trade-off: less detail than the parser's ("a key this file does not
//!   take" without saying which); the line and column point at it, and
//!   the person has the file open.

use std::fmt;
use std::ops::Range;

/// A place in a file, both counted from one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePlace {
    /// The line.
    pub line: usize,
    /// The column, in characters.
    pub column: usize,
}

impl FilePlace {
    /// The place of byte `offset` in `text`.
    #[must_use]
    pub fn of(text: &str, offset: usize) -> Self {
        let before = text.get(..offset).unwrap_or(text);
        let line_start = before.rfind('\n').map_or(0, |at| at.saturating_add(1));
        Self {
            line: before.matches('\n').count().saturating_add(1),
            column: before
                .get(line_start..)
                .map_or(0, |line| line.chars().count())
                .saturating_add(1),
        }
    }
}

/// What kind of problem stopped a TOML file from reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TomlProblem {
    /// Not TOML: an unclosed quote or bracket, a line without `=`, …
    NotToml,
    /// A key the file does not take.
    UnknownKey,
    /// A key given twice.
    DuplicateKey,
    /// A key the file needs is missing.
    MissingKey,
    /// A value of the wrong kind (a string where a table belongs, …).
    WrongKind,
}

/// Why a TOML file did not read: where, what, and which of the file's own
/// keys when that is known. Holds nothing taken from the file's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TomlFault {
    /// Where, when the parser said.
    pub at: Option<FilePlace>,
    /// What kind of problem.
    pub problem: TomlProblem,
    /// The key on that line, when it is one of the file's own keys.
    pub field: Option<&'static str>,
}

impl TomlFault {
    /// The fault a parser reported with `message` at `span` in `text`;
    /// `fields` are the keys the file may have (only those are named).
    #[must_use]
    pub fn of(
        message: &str,
        span: Option<Range<usize>>,
        text: &str,
        fields: &[&'static str],
    ) -> Self {
        let at = span.map(|span| FilePlace::of(text, span.start));
        let problem = Self::kind(message, text);
        let field = at.and_then(|place| field_on_line(text, place.line, fields));
        Self { at, problem, field }
    }

    /// A fault the client found itself (`field`'s value is of the wrong
    /// kind), with no place.
    #[must_use]
    pub const fn wrong_kind(field: &'static str) -> Self {
        Self {
            at: None,
            problem: TomlProblem::WrongKind,
            field: Some(field),
        }
    }

    /// The kind from serde's fixed wording at the start of the message
    /// (only the start is read; nothing of it is kept). A message in none
    /// of them is a syntax error when the text is not TOML, else a value
    /// a type refused.
    fn kind(message: &str, text: &str) -> TomlProblem {
        let message = message.trim_start();
        let starts = |words: &str| message.starts_with(words);
        if starts("unknown field") {
            TomlProblem::UnknownKey
        } else if starts("duplicate key") || starts("duplicate field") {
            TomlProblem::DuplicateKey
        } else if starts("missing field") {
            TomlProblem::MissingKey
        } else if text.parse::<toml_edit::DocumentMut>().is_err() {
            TomlProblem::NotToml
        } else {
            TomlProblem::WrongKind
        }
    }
}

/// The key on line `line` of `text` when it is one of `fields`.
fn field_on_line(text: &str, line: usize, fields: &[&'static str]) -> Option<&'static str> {
    let written = text.lines().nth(line.checked_sub(1)?)?;
    let key = written.split_once('=')?.0.trim().trim_matches('"');
    fields.iter().copied().find(|field| *field == key)
}

impl fmt::Display for TomlFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(at) = self.at {
            write!(f, "line {}, column {}: ", at.line, at.column)?;
        }
        match (self.problem, self.field) {
            (TomlProblem::NotToml, _) => f.write_str(
                "this is not TOML (look for an unclosed quote or bracket, or a line without `=`)",
            ),
            (TomlProblem::UnknownKey, _) => f.write_str("a key this file does not take"),
            (TomlProblem::DuplicateKey, _) => f.write_str("a key given twice"),
            (TomlProblem::MissingKey, _) => f.write_str("a key this file needs is missing"),
            (TomlProblem::WrongKind, Some("token")) => f.write_str(
                "`token` names the file that holds the token: token = { file = \"…\" } \
                 (never the token itself)",
            ),
            (TomlProblem::WrongKind, Some(field)) => {
                write!(f, "`{field}` has a value of the wrong kind")
            }
            (TomlProblem::WrongKind, None) => f.write_str("a value of the wrong kind"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_counts_lines_and_characters_from_one() {
        let text = "a = 1\n한글 = \"x\"\n";
        assert_eq!(FilePlace::of(text, 0), FilePlace { line: 1, column: 1 });
        let offset = text.find('=').unwrap();
        assert_eq!(
            FilePlace::of(text, offset),
            FilePlace { line: 1, column: 3 }
        );
        let second = text.rfind('=').unwrap();
        assert_eq!(
            FilePlace::of(text, second),
            FilePlace { line: 2, column: 4 }
        );
    }

    #[test]
    fn only_the_files_own_keys_are_named() {
        let text = "token = \"x\"\nsecret-looking-key = 1\n";
        assert_eq!(field_on_line(text, 1, &["token"]), Some("token"));
        assert_eq!(field_on_line(text, 2, &["token"]), None);
        assert_eq!(field_on_line(text, 9, &["token"]), None);
    }
}
