//! What git a node found (ADR 0023): reported with the project list, since
//! that is where a client first needs to know whether git calls can work.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A git version, as `git --version` reports it (`2.43.0`, `2.45.1` of
/// `2.45.1.windows.1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GitVersion {
    /// The first number.
    pub major: u16,
    /// The second.
    pub minor: u16,
    /// The third (0 when git printed two).
    pub patch: u16,
}

impl GitVersion {
    /// The oldest git the workspace calls work with: `--end-of-options`
    /// (2.24), `switch` and `restore` (2.23), `for-each-ref`'s
    /// `%(worktreepath)` (2.23); 2.25 is what the oldest supported long-term
    /// distributions ship (Ubuntu 20.04).
    pub const MINIMUM: Self = Self {
        major: 2,
        minor: 25,
        patch: 0,
    };

    /// The version in `git --version`'s output, or `None` when it has none.
    #[must_use]
    pub fn parse(output: &str) -> Option<Self> {
        let text = output.trim().strip_prefix("git version ")?;
        let mut numbers = text
            .split(|c: char| !c.is_ascii_digit())
            .take(3)
            .map(str::parse::<u16>);
        let major = numbers.next()?.ok()?;
        let minor = numbers.next()?.ok()?;
        let patch = numbers.next().and_then(Result::ok).unwrap_or(0);
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for GitVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Whether the node can run the git calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum GitSupport {
    /// A usable git.
    Found {
        /// Its version.
        version: GitVersion,
    },
    /// A git older than [`GitVersion::MINIMUM`]: git calls answer
    /// `unsupported`.
    TooOld {
        /// Its version.
        version: GitVersion,
        /// The oldest that works.
        minimum: GitVersion,
    },
    /// No `git` on the node's `PATH` (or it did not say its version).
    Missing,
    /// A state a newer node knows.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_parse_from_every_platform_s_git() {
        for (output, version) in [
            ("git version 2.43.0\n", (2, 43, 0)),
            ("git version 2.39.3 (Apple Git-146)", (2, 39, 3)),
            ("git version 2.45.1.windows.1", (2, 45, 1)),
            ("git version 2.25", (2, 25, 0)),
        ] {
            let parsed = GitVersion::parse(output).unwrap();
            assert_eq!(
                (parsed.major, parsed.minor, parsed.patch),
                version,
                "{output}"
            );
        }
        assert_eq!(GitVersion::parse("hub version 2.14"), None);
        assert!(GitVersion::parse("git version 2.24.9").unwrap() < GitVersion::MINIMUM);
        assert!(GitVersion::parse("git version 2.25.0").unwrap() >= GitVersion::MINIMUM);
    }
}
