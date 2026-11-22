//! The bounded strings of schedules, saved prompts and hooks (protocol 11):
//! their names, the prompt text they send, a time zone's name and a hook
//! sender's delivery key.
//!
//! [Decision Log]
//! - Purpose: a prompt a schedule sends is logged with every run and
//!   queued into a session (whose queue holds 128 KiB in all); a name is a
//!   line on a page; a zone's name is looked up in the node's database.
//!   Each has a shape a client can check before it sends.
//! - Choice: the conversation and workspace modules' pattern: one rule per
//!   kind ([`AutomationText`]), one error ([`InvalidAutomationText`]), a
//!   private macro declaring each newtype, decoded through its parse. Kept
//!   apart from `workspace::text` (whose rules are git's) so the two
//!   tracks touching them do not share a file.
//! - `PromptText` is 16 KiB: agent-orchestrator's bound for a Cue, and an
//!   eighth of a session's queue, so a scheduled prompt always fits one.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// Which bounded string, and so which rule applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutomationText {
    /// A schedule's or a hook's name: 1 to 80 characters, one line.
    Name,
    /// A saved prompt's name: 1 to 64 characters, one line.
    PromptName,
    /// A prompt's text: not blank, at most 16 KiB, no control character
    /// but tab and line ends.
    PromptText,
    /// An IANA time zone's name: its shape only.
    TimeZone,
    /// A hook sender's delivery key: 1 to 128 visible ASCII characters.
    DeliveryKey,
}

impl fmt::Display for AutomationText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Name => "name",
            Self::PromptName => "prompt name",
            Self::PromptText => "prompt",
            Self::TimeZone => "time zone name",
            Self::DeliveryKey => "delivery key",
        })
    }
}

/// A string refused for its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidAutomationText {
    /// What it was meant to be.
    pub kind: AutomationText,
    /// What is wrong, described.
    pub problem: &'static str,
}

impl fmt::Display for InvalidAutomationText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a {} {}", self.kind, self.problem)
    }
}

impl std::error::Error for InvalidAutomationText {}

impl AutomationText {
    /// The most bytes a [`PromptText`] holds.
    pub const PROMPT_MAX: usize = 16 * 1024;

    /// Check `text` against this kind's rule; the text to keep (trimmed
    /// for names, as given otherwise).
    fn parse(self, text: &str) -> Result<Arc<str>, &'static str> {
        match self {
            Self::Name => line(text.trim(), 80).map(|()| Arc::from(text.trim())),
            Self::PromptName => line(text.trim(), 64).map(|()| Arc::from(text.trim())),
            Self::PromptText => {
                if text.trim().is_empty() {
                    return Err("cannot be empty");
                }
                Self::validate_prompt_bytes(text)?;
                Ok(Arc::from(text))
            }
            Self::TimeZone => zone(text).map(|()| Arc::from(text)),
            Self::DeliveryKey => {
                if text.is_empty() || text.len() > 128 {
                    return Err("is 1 to 128 characters");
                }
                if !text.bytes().all(|byte| byte.is_ascii_graphic()) {
                    return Err("is visible ASCII only");
                }
                Ok(Arc::from(text))
            }
        }
    }

    /// The shared transport bounds. Each caller owns its policy on blank
    /// text because a settings patch can use blank as an explicit clear.
    pub(crate) fn validate_prompt_bytes(text: &str) -> Result<(), &'static str> {
        if text.len() > Self::PROMPT_MAX {
            return Err("is at most 16 KiB");
        }
        if text.chars().any(|character| {
            character.is_control() && !matches!(character, '\t' | '\n' | '\r')
        }) {
            return Err("cannot contain control characters other than tab and line ends");
        }
        Ok(())
    }
}

/// One line: not blank, at most `max` characters, no control character.
fn line(text: &str, max: usize) -> Result<(), &'static str> {
    if text.is_empty() {
        return Err("cannot be empty");
    }
    if text.chars().any(char::is_control) {
        return Err("is one line without control characters");
    }
    if text.chars().count() > max {
        return Err(if max == 64 {
            "is at most 64 characters"
        } else {
            "is at most 80 characters"
        });
    }
    Ok(())
}

/// An IANA name's shape (`Europe/London`, `America/Argentina/Salta`,
/// `Etc/GMT+5`, `UTC`): ASCII letters, digits and `/ _ - +`, parts not
/// empty, `.` or `..`. Whether the zone exists is the node's to say.
fn zone(text: &str) -> Result<(), &'static str> {
    if text.is_empty() || text.len() > 64 {
        return Err("is 1 to 64 characters");
    }
    if !text
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-' | b'+'))
    {
        return Err("is ASCII letters, digits and `/ _ - +` (an IANA name such as Europe/London)");
    }
    if text.split('/').any(str::is_empty) {
        return Err("has no empty part");
    }
    Ok(())
}

/// Declares a bounded string of one [`AutomationText`] kind.
macro_rules! automation_text {
    ($($(#[$meta:meta])* $name:ident = $kind:ident;)*) => {$(
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub struct $name(Arc<str>);

        impl $name {
            /// The text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl FromStr for $name {
            type Err = InvalidAutomationText;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                AutomationText::$kind
                    .parse(text)
                    .map(Self)
                    .map_err(|problem| InvalidAutomationText {
                        kind: AutomationText::$kind,
                        problem,
                    })
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        crate::key::serde_via_text!($name);
    )*};
}

automation_text! {
    /// What a schedule or a hook is called on a page.
    ScheduleName = Name;
    /// What a saved prompt is called; unique within its project (or the
    /// machine's own).
    PromptName = PromptName;
    /// The text a schedule, a saved prompt or a hook sends.
    PromptText = PromptText;
    /// An IANA time zone's name, as a person chose it (`Asia/Seoul`).
    TimeZoneName = TimeZone;
    /// The `Vessel-Delivery` header a hook's sender sets, so a delivery
    /// sent twice runs once.
    DeliveryKey = DeliveryKey;
}

impl PromptText {
    /// Reference default shared by construction and default-value projection.
    pub(crate) const RESUME_DEFAULT: &'static str = "Resume working.";

    /// A trusted nonblank reference literal, within the existing byte bound.
    /// External input still passes the ordinary validating parser.
    pub(crate) fn resume_default() -> Self {
        Self(Arc::from(Self::RESUME_DEFAULT))
    }
}

impl TimeZoneName {
    /// Coordinated Universal Time.
    #[must_use]
    pub fn utc() -> Self {
        Self(Arc::from("UTC"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_lines() {
        let name: ScheduleName = "  nightly review ".parse().unwrap();
        assert_eq!(name.as_str(), "nightly review");
        assert!("".parse::<ScheduleName>().is_err());
        assert!("a\nb".parse::<ScheduleName>().is_err());
        assert!("x".repeat(81).parse::<ScheduleName>().is_err());
        assert!("x".repeat(80).parse::<ScheduleName>().is_ok());
        assert!("x".repeat(65).parse::<PromptName>().is_err());
    }

    #[test]
    fn prompts_are_bounded_text() {
        let text: PromptText = "Review the open pull requests.\n\nBe brief."
            .parse()
            .unwrap();
        assert!(text.as_str().ends_with("brief."));
        assert!(" \n ".parse::<PromptText>().is_err());
        assert!("a\0b".parse::<PromptText>().is_err());
        assert!("bell\u{7}".parse::<PromptText>().is_err());
        assert!("tab\tand\r\nlines".parse::<PromptText>().is_ok());
        assert!(
            "x".repeat(AutomationText::PROMPT_MAX + 1)
                .parse::<PromptText>()
                .is_err()
        );
        assert!(
            "x".repeat(AutomationText::PROMPT_MAX)
                .parse::<PromptText>()
                .is_ok()
        );
    }

    #[test]
    fn zones_have_an_iana_shape() {
        for good in ["UTC", "Asia/Seoul", "America/Argentina/Salta", "Etc/GMT+5"] {
            assert!(good.parse::<TimeZoneName>().is_ok(), "{good}");
        }
        for bad in [
            "",
            "Europe//London",
            "/UTC",
            "../etc",
            "Europe/Lon don",
            "Zone\u{e9}",
        ] {
            assert!(bad.parse::<TimeZoneName>().is_err(), "{bad}");
        }
        assert!(serde_json::from_str::<TimeZoneName>(r#""a b""#).is_err());
    }

    #[test]
    fn delivery_keys_are_visible_ascii() {
        assert!("evt_123:retry-2".parse::<DeliveryKey>().is_ok());
        assert!("has space".parse::<DeliveryKey>().is_err());
        assert!("x".repeat(129).parse::<DeliveryKey>().is_err());
    }
}
