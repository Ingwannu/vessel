//! The bounded values of the forge calls (protocol 11, ADR 0029): a pull
//! request's number, a host and a repository's path as a remote names
//! them, a login, a title, a body, an address, a check's name.
//!
//! [Decision Log]
//! - Purpose: these cross the wire inside `SessionInfo`, which every
//!   client lists every two seconds, and some end up in a request's path
//!   (`/repos/{path}/pulls/{number}`). Each is a type whose only
//!   constructor checks it (AGENTS.md section 2).
//! - Constraint: most of them come from a forge, which Vessel does not
//!   control: a check's name with a line break, a title past any bound, an
//!   `http` details address. Strict decoding alone would let one such
//!   value make a whole session list fail to decode for every client
//!   (the contract's review, finding 5).
//! - Choice: strict `FromStr` (what decoding uses), and, for each value a
//!   forge supplies, a lossy constructor the node uses when it builds one
//!   from a forge's answer: it replaces what the rule refuses, cuts what is
//!   too long, and gives `None` for an optional value it cannot make (an
//!   address that is not `https`). So what the node sends always decodes.
//! - Repository paths take GitLab's nested groups (2 to 8 segments) now,
//!   so the trait shaped for GitLab needs no wire change later.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::key::serde_via_text;

/// What is wrong with a forge value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidForgeValue {
    /// What it was meant to be.
    pub kind: &'static str,
    /// The rule it broke.
    pub rule: &'static str,
}

impl fmt::Display for InvalidForgeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a valid {}: {}", self.kind, self.rule)
    }
}

impl std::error::Error for InvalidForgeValue {}

const fn invalid(kind: &'static str, rule: &'static str) -> InvalidForgeValue {
    InvalidForgeValue { kind, rule }
}

/// A pull request's number in its repository (GitLab's merge request
/// `iid`): 1 to 2^53 − 1, what a page's JavaScript holds exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct PullNumber(u64);

impl PullNumber {
    /// The largest number a page holds exactly.
    pub const MAX: u64 = (1 << 53) - 1;

    /// `number`, when it is one.
    ///
    /// # Errors
    /// Zero, or past [`Self::MAX`].
    pub const fn new(number: u64) -> Result<Self, InvalidForgeValue> {
        if number == 0 || number > Self::MAX {
            return Err(invalid("pull request number", "1 to 2^53 - 1"));
        }
        Ok(Self(number))
    }

    /// The number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for PullNumber {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(u64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for PullNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Declares a checked string: `FromStr` by `$rule`, serde through it.
macro_rules! forge_text {
    ($(#[$meta:meta])* $name:ident, $kind:literal, $rule:path) => {
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
            type Err = InvalidForgeValue;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                $rule(text)
                    .map(|()| Self(Arc::from(text)))
                    .map_err(|rule| invalid($kind, rule))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        serde_via_text!($name);
    };
}

forge_text!(
    /// A forge's host as a remote names it (`github.com`, `git.example.org:8443`).
    ForgeHost,
    "forge host",
    host_rule
);
forge_text!(
    /// A repository's path on its forge: `owner/name`, or GitLab's
    /// `group/subgroup/name`.
    RepoPath,
    "repository path",
    repo_rule
);
forge_text!(
    /// A person's or a bot's login on a forge.
    ForgeLogin,
    "forge login",
    login_rule
);
forge_text!(
    /// A pull request's title: one line.
    PullTitle,
    "pull request title",
    title_rule
);
forge_text!(
    /// A pull request's description.
    PullBody,
    "pull request description",
    body_rule
);
forge_text!(
    /// A page on a forge: `https` (or `http` on a loopback host, the
    /// tests' fake), with no user information.
    ForgeUrl,
    "forge address",
    url_rule
);
forge_text!(
    /// A check's or a status context's name.
    CheckName,
    "check name",
    check_rule
);
forge_text!(
    /// A repository's description on its forge: one line (ADR 0088).
    RepoDescription,
    "repository description",
    description_rule
);
forge_text!(
    /// What `forge.repos` searches GitHub for: one line, empty for the
    /// signed-in account's own repositories (ADR 0088).
    RepoQuery,
    "repository query",
    query_rule
);

/// The most characters of a title.
const TITLE: usize = 256;
/// The most bytes of a description.
const BODY: usize = 64 * 1024;
/// The most characters of a check's name.
const CHECK: usize = 256;
/// The most characters of a repository's description (GitHub's own).
const DESCRIPTION: usize = 350;
/// The most characters of a repository query.
const QUERY: usize = 256;
/// The most bytes of an address.
const URL: usize = 2048;

fn host_rule(text: &str) -> Result<(), &'static str> {
    let (name, port) = match text.rsplit_once(':') {
        Some((name, port)) => (name, Some(port)),
        None => (text, None),
    };
    if name.is_empty() || name.len() > 253 {
        return Err("1 to 253 bytes");
    }
    let label_ok = |label: &str| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    };
    if !name.split('.').all(label_ok) {
        return Err("lower-case labels of a-z, 0-9 and -");
    }
    if let Some(port) = port
        && port.parse::<u16>().map_or(true, |port| port == 0)
    {
        return Err("a port from 1 to 65535");
    }
    Ok(())
}

fn repo_rule(text: &str) -> Result<(), &'static str> {
    let segments: Vec<&str> = text.split('/').collect();
    if !(2..=8).contains(&segments.len()) {
        return Err("2 to 8 segments");
    }
    for segment in segments {
        if segment.is_empty() || segment.len() > 100 {
            return Err("segments of 1 to 100 bytes");
        }
        if segment == "." || segment == ".." || segment.starts_with('-') {
            return Err("no `.`, `..` or leading `-` segment");
        }
        if !segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err("segments of A-Z, a-z, 0-9, `.`, `_` and `-`");
        }
    }
    Ok(())
}

fn login_rule(text: &str) -> Result<(), &'static str> {
    let name = text.strip_suffix("[bot]").unwrap_or(text);
    if name.is_empty() || text.len() > 100 {
        return Err("1 to 100 bytes");
    }
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err("A-Z, a-z, 0-9, `-`, `_`, `.` and a trailing `[bot]`");
    }
    Ok(())
}

fn line(text: &str, max: usize) -> Result<(), &'static str> {
    if text.trim().is_empty() {
        return Err("not blank");
    }
    if text.chars().any(char::is_control) {
        return Err("one line, no control character");
    }
    if text.chars().count() > max {
        return Err("too long");
    }
    Ok(())
}

fn title_rule(text: &str) -> Result<(), &'static str> {
    line(text, TITLE)
}

fn check_rule(text: &str) -> Result<(), &'static str> {
    line(text, CHECK)
}

fn description_rule(text: &str) -> Result<(), &'static str> {
    line(text, DESCRIPTION)
}

/// A query may be empty (Paseo's empty search lists the account's own).
fn query_rule(text: &str) -> Result<(), &'static str> {
    if text.chars().any(char::is_control) {
        return Err("one line, no control character");
    }
    if text.chars().count() > QUERY {
        return Err("too long");
    }
    Ok(())
}

fn body_rule(text: &str) -> Result<(), &'static str> {
    if text.len() > BODY {
        return Err("at most 64 KiB");
    }
    if text.contains('\0') {
        return Err("no NUL");
    }
    Ok(())
}

fn url_rule(text: &str) -> Result<(), &'static str> {
    if text.len() > URL {
        return Err("at most 2 KiB");
    }
    if text
        .chars()
        .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err("no white space or control character");
    }
    let (secure, rest) = if let Some(rest) = text.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = text.strip_prefix("http://") {
        (false, rest)
    } else {
        return Err("an https address");
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return Err("a host, and no user information");
    }
    let host = authority
        .rsplit_once(':')
        .map_or(authority, |(host, _port)| host);
    if !secure && !matches!(host, "127.0.0.1" | "localhost" | "[::1]") {
        return Err("an https address (http only on loopback)");
    }
    Ok(())
}

/// One line made of `text`: control and format characters (Unicode `Cc`
/// and the bidirectional and zero-width ones) become spaces, runs of white
/// space one space, at most `max` characters; `None` when nothing is left.
fn lossy_line(text: &str, max: usize) -> Option<String> {
    let mut out = String::new();
    let mut space = false;
    for character in text.chars() {
        let character = if character.is_control() || is_format(character) {
            ' '
        } else {
            character
        };
        if character.is_whitespace() {
            space = !out.is_empty();
            continue;
        }
        if space {
            out.push(' ');
            space = false;
        }
        out.push(character);
        if out.chars().count() >= max {
            break;
        }
    }
    (!out.is_empty()).then_some(out)
}

/// Unicode format characters a quoted text must not carry: they reorder
/// or hide what a reader sees (bidirectional controls, zero-width
/// characters, the byte-order mark).
#[must_use]
pub fn is_format(character: char) -> bool {
    matches!(
        character,
        '\u{00ad}'
            | '\u{061c}'
            | '\u{180e}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206f}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}'
    )
}

impl PullTitle {
    /// A title made of a forge's `text`, whatever it holds (the contract's
    /// review, finding 5): one line, at most 256 characters, `(untitled)`
    /// when nothing is left.
    #[must_use]
    pub fn lossy(text: &str) -> Self {
        Self(Arc::from(
            lossy_line(text, TITLE).unwrap_or_else(|| "(untitled)".to_owned()),
        ))
    }
}

impl CheckName {
    /// A check's name made of a forge's `text`: one line, at most 256
    /// characters, `(unnamed)` when nothing is left.
    #[must_use]
    pub fn lossy(text: &str) -> Self {
        Self(Arc::from(
            lossy_line(text, CHECK).unwrap_or_else(|| "(unnamed)".to_owned()),
        ))
    }
}

impl RepoDescription {
    /// A description made of a forge's `text`: one line, at most 350
    /// characters; `None` when nothing is left.
    #[must_use]
    pub fn lossy(text: &str) -> Option<Self> {
        lossy_line(text, DESCRIPTION).map(|line| Self(Arc::from(line)))
    }
}

impl RepoQuery {
    /// The text without the white space around it, as the node searches.
    #[must_use]
    pub fn trimmed(&self) -> &str {
        self.0.trim()
    }
}

impl Default for RepoQuery {
    /// The empty query: the account's own repositories.
    fn default() -> Self {
        Self(Arc::from(""))
    }
}

impl ForgeUrl {
    /// `text` when it is an address this type holds; `None` otherwise (a
    /// forge's `http` details page, an address with credentials).
    #[must_use]
    pub fn lossy(text: &str) -> Option<Self> {
        text.parse().ok()
    }
}

impl ForgeLogin {
    /// `text` when it is a login; `None` otherwise.
    #[must_use]
    pub fn lossy(text: &str) -> Option<Self> {
        text.parse().ok()
    }
}

impl PullBody {
    /// The empty description.
    #[must_use]
    pub fn empty() -> Self {
        Self(Arc::from(""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_positive_and_fit_a_page() {
        assert!(PullNumber::new(0).is_err());
        assert!(PullNumber::new(PullNumber::MAX + 1).is_err());
        assert_eq!(PullNumber::new(42).unwrap().get(), 42);
        assert!(serde_json::from_str::<PullNumber>("0").is_err());
        assert_eq!(
            serde_json::to_string(&PullNumber::new(7).unwrap()).unwrap(),
            "7"
        );
    }

    #[test]
    fn hosts_and_paths_are_what_a_remote_names() {
        for good in ["github.com", "git.example.org:8443", "127.0.0.1:9"] {
            assert!(good.parse::<ForgeHost>().is_ok(), "{good}");
        }
        for bad in [
            "",
            "GitHub.com",
            "-a.com",
            "a..b",
            "a.com:0",
            "a.com:x",
            "a b",
        ] {
            assert!(bad.parse::<ForgeHost>().is_err(), "{bad}");
        }
        for good in ["owner/name", "group/sub/name", "o/n.git-x_y"] {
            assert!(good.parse::<RepoPath>().is_ok(), "{good}");
        }
        for bad in [
            "name",
            "o/",
            "/n",
            "o/../n",
            "o/-n",
            "o/n?x",
            "a/b/c/d/e/f/g/h/i",
        ] {
            assert!(bad.parse::<RepoPath>().is_err(), "{bad}");
        }
    }

    #[test]
    fn addresses_are_https_without_credentials() {
        assert!("https://github.com/o/n/pull/1".parse::<ForgeUrl>().is_ok());
        assert!("http://127.0.0.1:8080/o/n".parse::<ForgeUrl>().is_ok());
        for bad in [
            "http://github.com/o/n",
            "https://user:token@github.com/o",
            "https://",
            "ftp://x/y",
            "https://a.com/x y",
        ] {
            assert!(bad.parse::<ForgeUrl>().is_err(), "{bad}");
            assert!(ForgeUrl::lossy(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn a_forge_s_text_is_made_into_one_that_decodes() {
        let hostile = "Fix\nthe\u{202e}gnissap \u{200b} tests\t\u{0007}";
        let title = PullTitle::lossy(hostile);
        assert_eq!(title.as_str(), "Fix the gnissap tests");
        let json = serde_json::to_string(&title).unwrap();
        assert_eq!(serde_json::from_str::<PullTitle>(&json).unwrap(), title);
        assert_eq!(PullTitle::lossy("\n\u{200b}").as_str(), "(untitled)");
        let long = "x".repeat(1000);
        assert_eq!(CheckName::lossy(&long).as_str().chars().count(), 256);
        assert!("line\nbreak".parse::<CheckName>().is_err());
        assert!(ForgeLogin::lossy("dependabot[bot]").is_some());
        assert!(ForgeLogin::lossy("bad login").is_none());
    }
}
