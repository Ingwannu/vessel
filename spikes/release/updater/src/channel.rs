//! A release channel's name (`preview`).
//!
//! A channel is a line of builds that one copy of the app follows: it is
//! compiled in, names the feed's manifest (`<feed>/<channel>.json`) and
//! is repeated inside the signed manifest, so a manifest signed for one
//! channel cannot be served as another's. Zed has four fixed channels
//! (dev, nightly, preview, stable); Vessel has only `preview` for now and
//! keeps the set open (a name, not an enum) because the owner decides
//! later which others exist, and adding one must not need a new app.

use std::{fmt, str::FromStr};

/// The longest channel name; it becomes part of a URL and a file name.
const MAX_LEN: usize = 32;

/// A channel name: a lowercase ASCII letter, then lowercase letters,
/// digits or `-`, at most 32 in all.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Channel(String);

/// Why a text is not a channel name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a channel is a lowercase letter, then lowercase letters, digits or '-', at most 32")]
pub struct ChannelError;

impl Channel {
    /// The channel the app's builds follow until the owner names others.
    #[must_use]
    pub fn preview() -> Self {
        Self("preview".to_owned())
    }

    /// The name, as it appears in the feed.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Channel {
    type Err = ChannelError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut bytes = text.bytes();
        let starts_with_letter = bytes.next().is_some_and(|byte| byte.is_ascii_lowercase());
        let rest_allowed =
            bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        if starts_with_letter && rest_allowed && text.len() <= MAX_LEN {
            Ok(Self(text.to_owned()))
        } else {
            Err(ChannelError)
        }
    }
}

impl TryFrom<String> for Channel {
    type Error = ChannelError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Channel> for String {
    fn from(channel: Channel) -> Self {
        channel.0
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_that_are_and_are_not_channels() {
        for good in ["preview", "stable", "a", "beta-2"] {
            assert!(good.parse::<Channel>().is_ok(), "{good:?} refused");
        }
        for bad in [
            "",
            "Preview",
            "2beta",
            "-a",
            "a/b",
            "a.b",
            "../x",
            "a b",
            &"a".repeat(33),
        ] {
            assert!(bad.parse::<Channel>().is_err(), "{bad:?} accepted");
        }
        assert_eq!(Channel::preview().as_str(), "preview");
    }
}
