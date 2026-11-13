//! The bounded strings of protocol 11's agents (ADR 0032): what a person
//! writes into an added agent's definition, and what the ACP registry and
//! an agent's handshake say of it.
//!
//! [Decision Log]
//! - Purpose: these strings come from a person, from the network (the
//!   registry) or from an agent, and end up in a file of the node's
//!   (`agents.toml`), on a command line, or on one line of a page. Each is
//!   parsed once, where it is decoded, so nothing downstream holds an
//!   unchecked or unbounded string.
//! - Alternatives: `String`s checked where used (each use repeats or
//!   forgets the check); reusing the conversation's words (an agent's label
//!   could then be passed where a command name is wanted).
//! - Choice: one newtype per string with its own rule, built only by
//!   `FromStr` and decoded through it (`serde_via_text!`), as the
//!   conversation's words are; a value past its bound is a decode error.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// Why a string was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidAgentText {
    /// Which string.
    pub what: &'static str,
    /// The rule it breaks, for people.
    pub rule: &'static str,
}

impl fmt::Display for InvalidAgentText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} must be {}", self.what, self.rule)
    }
}

impl std::error::Error for InvalidAgentText {}

/// Declares one bounded string: an `Arc<str>` that only `FromStr` builds,
/// with `$check` deciding.
macro_rules! agent_text {
    ($(#[$meta:meta])* $name:ident, $what:literal, $rule:literal, $check:expr) => {
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
            type Err = InvalidAgentText;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                let check: fn(&str) -> bool = $check;
                if check(text) {
                    Ok(Self(Arc::from(text)))
                } else {
                    Err(InvalidAgentText { what: $what, rule: $rule })
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

agent_text! {
    /// What a person calls an added agent (`Goose`), shown on the page.
    AgentLabel,
    "an agent's label",
    "1 to 80 characters on one line",
    |text| line(text, 80)
}

agent_text! {
    /// An added agent's program as written: a name looked up on the node's
    /// search path (`goose`, `npx`), or an absolute path.
    AgentProgram,
    "an agent's program",
    "1 to 1,024 bytes without a control character, not starting or ending with white space",
    |text| {
        !text.is_empty()
            && text.len() <= 1024
            && text.trim() == text
            && !text.chars().any(char::is_control)
    }
}

agent_text! {
    /// An agent's version as it reports it (ACP `agentInfo.version`, or the
    /// first line of a known agent's `--version`).
    AgentVersion,
    "an agent's version",
    "1 to 64 characters on one line",
    |text| line(text, 64)
}

agent_text! {
    /// An agent's id in the ACP registry (`goose`).
    RegistryId,
    "a registry id",
    "1 to 128 characters of `a-z`, `0-9`, `.`, `_` and `-`, starting with a letter or digit",
    |text| {
        text.len() <= 128
            && text
                .bytes()
                .next()
                .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
            && text.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
            })
    }
}

agent_text! {
    /// A version in the ACP registry: the registry's own, or an entry's.
    RegistryVersion,
    "a registry version",
    "1 to 64 characters of letters, digits, `.`, `_`, `+` and `-`, starting with a letter or digit",
    |text| {
        text.len() <= 64
            && text.bytes().next().is_some_and(|first| first.is_ascii_alphanumeric())
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
    }
}

agent_text! {
    /// The id of a sign-in method an agent offers (ACP `authMethods[].id`),
    /// echoed back as given.
    AuthMethodId,
    "a sign-in method's id",
    "1 to 128 characters on one line",
    |text| line(text, 128)
}

agent_text! {
    /// A SHA-256 digest, as 64 hexadecimal digits (lower case).
    Sha256Hex,
    "a SHA-256 digest",
    "64 lower-case hexadecimal digits",
    |text| text.len() == 64 && text.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Not blank, one line without control characters, at most `max`
/// characters.
fn line(text: &str, max: usize) -> bool {
    !text.trim().is_empty() && !text.chars().any(char::is_control) && text.chars().count() <= max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_string_keeps_its_rule() {
        assert!("Goose".parse::<AgentLabel>().is_ok());
        assert!("two\nlines".parse::<AgentLabel>().is_err());
        assert!("x".repeat(81).parse::<AgentLabel>().is_err());

        assert!("npx".parse::<AgentProgram>().is_ok());
        assert!("/opt/agent/bin/agent".parse::<AgentProgram>().is_ok());
        assert!(" npx".parse::<AgentProgram>().is_err());
        assert!("".parse::<AgentProgram>().is_err());
        assert!("a\u{0}b".parse::<AgentProgram>().is_err());

        assert!("goose".parse::<RegistryId>().is_ok());
        assert!("claude-code-acp".parse::<RegistryId>().is_ok());
        assert!("Goose".parse::<RegistryId>().is_err());
        assert!("-x".parse::<RegistryId>().is_err());

        assert!("1.9.0".parse::<RegistryVersion>().is_ok());
        assert!("1.0.0+build.2".parse::<RegistryVersion>().is_ok());
        assert!("..".parse::<RegistryVersion>().is_err());
        assert!("1 0".parse::<RegistryVersion>().is_err());

        assert!("a".repeat(64).parse::<Sha256Hex>().is_ok());
        assert!("A".repeat(64).parse::<Sha256Hex>().is_err());
        assert!("a".repeat(63).parse::<Sha256Hex>().is_err());
    }

    #[test]
    fn a_bad_one_does_not_decode() {
        assert!(serde_json::from_str::<AgentLabel>(r#""ok""#).is_ok());
        assert!(serde_json::from_str::<AgentLabel>(r#""""#).is_err());
        assert!(serde_json::from_str::<RegistryId>(r#""UPPER""#).is_err());
    }
}
