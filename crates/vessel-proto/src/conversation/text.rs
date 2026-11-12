//! The bounded words of the conversation (protocol 10): a command's name,
//! an attachment's file name and type, a cost's unit, a question field's
//! key. Each is parsed once, where a frame is decoded, so nothing
//! downstream holds an unchecked string.
//!
//! [Decision Log]
//! - Purpose: these strings come from agents and clients and end up in
//!   file names (an attachment is stored under its name, so an agent sees
//!   `report.pdf`), in a prompt the node sends (`/name`), and on one line
//!   of a page. A value that is fine in one of those and not in another is
//!   how a path escapes its folder or a line breaks a card.
//! - Alternatives: `String`s checked where used (each use repeats or
//!   forgets the check); one shared "label" type (an attachment name could
//!   then be passed as a command name).
//! - Choice: one newtype per word with its own rule, built only by
//!   `FromStr` and decoded through it (`serde_via_text!`), so a bad value
//!   on the wire or in the store is a decode error, never a value.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// Why a word was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidText {
    /// Which word.
    pub what: &'static str,
    /// The rule it breaks, for people.
    pub rule: &'static str,
}

impl fmt::Display for InvalidText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} must be {}", self.what, self.rule)
    }
}

impl std::error::Error for InvalidText {}

/// Declares a word: an `Arc<str>` (cheap to clone into many events) that
/// only `FromStr` builds, with `$check` deciding.
macro_rules! word {
    ($(#[$meta:meta])* $name:ident, $what:literal, $rule:literal, $check:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub struct $name(Arc<str>);

        impl $name {
            /// The word.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl FromStr for $name {
            type Err = InvalidText;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                let check: fn(&str) -> bool = $check;
                if check(text) {
                    Ok(Self(Arc::from(text)))
                } else {
                    Err(InvalidText { what: $what, rule: $rule })
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        crate::key::serde_via_text!($name);
    };
}

word! {
    /// A command the agent offers, without its `/` (`compact`, `review`).
    /// Sent back as the start of a prompt, `/name`, so it has no white
    /// space: what follows the first space is the command's input.
    CommandName,
    "a command name",
    "1 to 128 characters, without white space or a leading `/`",
    |text| {
        let chars = text.chars().count();
        (1..=128).contains(&chars)
            && !text.starts_with('/')
            && !text.chars().any(|c| c.is_whitespace() || c.is_control())
    }
}

word! {
    /// An attachment's file name as the person's file had it. The node
    /// stores the bytes under it (in a folder of the attachment's own id),
    /// so it is one path component on every platform.
    AttachmentName,
    "an attachment name",
    "a file name of 1 to 255 bytes: no `/`, `\\` or control character, not `.` or `..`, \
     no leading or trailing space or dot, and not a Windows device name",
    file_name
}

word! {
    /// A media type without parameters, `type/subtype`, lower case
    /// (`image/png`, `application/pdf`).
    MimeType,
    "a media type",
    "`type/subtype` in lower-case token characters, at most 127 bytes, without parameters",
    |text| {
        let Some((kind, sub)) = text.split_once('/') else {
            return false;
        };
        text.len() <= 127 && token(kind) && token(sub)
    }
}

word! {
    /// What a cost is counted in: an ISO 4217 code (`USD`) or the agent's
    /// own unit (`credits`).
    CostUnit,
    "a cost unit",
    "1 to 16 letters, digits or `_`",
    |text| {
        (1..=16).contains(&text.len())
            && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }
}

word! {
    /// The key of one field of a question's form, as the agent names it
    /// (`question_1`). Echoed back in an answer.
    FieldName,
    "a field name",
    "1 to 128 characters, without control characters",
    |text| {
        let chars = text.chars().count();
        (1..=128).contains(&chars) && !text.chars().any(char::is_control)
    }
}

impl MimeType {
    /// Its top-level type (`image` for `image/png`).
    #[must_use]
    pub fn top(&self) -> &str {
        self.0.split_once('/').map_or(&*self.0, |(top, _)| top)
    }
}

/// A lower-case RFC 6838 restricted name.
fn token(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|c| {
            c.is_ascii_lowercase()
                || c.is_ascii_digit()
                || matches!(c, '!' | '#' | '$' | '&' | '-' | '^' | '_' | '.' | '+')
        })
}

/// One path component that every platform the node runs on takes as a
/// plain file name.
fn file_name(text: &str) -> bool {
    if text.is_empty() || text.len() > 255 || text == "." || text == ".." {
        return false;
    }
    if text.starts_with([' ', '.']) || text.ends_with([' ', '.']) {
        return false;
    }
    if text.chars().any(|c| {
        c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    }) {
        return false;
    }
    !windows_device(text)
}

/// `CON`, `NUL`, `COM1`, `LPT9` and their like, with or without an
/// extension: Windows opens the device, not a file, for any of them.
fn windows_device(text: &str) -> bool {
    let stem = text.split('.').next().unwrap_or(text).to_ascii_uppercase();
    let stem = stem.trim_end();
    matches!(stem, "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|rest| {
                let mut digits = rest.chars();
                matches!(
                    (digits.next(), digits.next()),
                    (Some('0'..='9' | '¹' | '²' | '³'), None)
                )
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_names_have_no_slash_and_no_space() {
        assert!("compact".parse::<CommandName>().is_ok());
        assert!("review-pr".parse::<CommandName>().is_ok());
        assert!("/compact".parse::<CommandName>().is_err());
        assert!("two words".parse::<CommandName>().is_err());
        assert!("".parse::<CommandName>().is_err());
        assert!("a".repeat(129).parse::<CommandName>().is_err());
    }

    #[test]
    fn attachment_names_stay_one_file() {
        for good in ["report.pdf", "스크린샷 1.png", "a", "notes.tar.gz"] {
            assert!(good.parse::<AttachmentName>().is_ok(), "{good}");
        }
        for bad in [
            "", ".", "..", "../x", "a/b", "a\\b", "x\n", " lead", "trail.", ".hidden", "CON",
            "con.txt", "COM1", "lpt9.log", "a:b",
        ] {
            assert!(bad.parse::<AttachmentName>().is_err(), "{bad:?}");
        }
        assert!("COM10".parse::<AttachmentName>().is_ok(), "only one digit");
        assert!("x".repeat(256).parse::<AttachmentName>().is_err());
    }

    #[test]
    fn media_types_are_two_lower_case_tokens() {
        let png: MimeType = "image/png".parse().unwrap();
        assert_eq!(png.top(), "image");
        assert!("application/vnd.ms-excel".parse::<MimeType>().is_ok());
        for bad in [
            "image",
            "Image/PNG",
            "text/plain; charset=utf-8",
            "/png",
            "a/",
        ] {
            assert!(bad.parse::<MimeType>().is_err(), "{bad}");
        }
    }

    #[test]
    fn a_bad_word_does_not_decode() {
        assert!(serde_json::from_str::<AttachmentName>(r#""../etc""#).is_err());
        let unit: CostUnit = serde_json::from_str(r#""USD""#).unwrap();
        assert_eq!(serde_json::to_string(&unit).unwrap(), r#""USD""#);
        assert!(serde_json::from_str::<CostUnit>(r#""US Dollar""#).is_err());
        assert!(serde_json::from_str::<FieldName>(r#""question_1""#).is_ok());
    }
}
