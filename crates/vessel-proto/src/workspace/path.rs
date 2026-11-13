//! [`RelPath`]: a path inside a project or a worktree, as a client names it.
//!
//! [Decision Log]
//! - Purpose: every file and git call names a path inside a root the node
//!   chose (ADR 0027). A client must not be able to name anything else:
//!   not by `..`, not by an absolute path, not by a drive or a UNC prefix,
//!   and not the repository's own `.git`.
//! - Constraint: one wire, three platforms. A Windows node would read `\`
//!   and `C:` as separators and roots; a Unix one would not, so the same
//!   text could mean two things.
//! - Alternatives: a `PathBuf` checked by the node (every handler repeats
//!   the check; a path that decodes is not known to be safe anywhere
//!   else); accepting any text and only resolving (resolution is the
//!   second wall, not the first).
//! - Choice: a type whose parse refuses everything that is not a plain
//!   relative path with `/` between parts, so the node joins parts one by
//!   one onto the root and only then resolves links and checks that the
//!   result stays inside (`vessel-workspace`'s `paths`). Two walls: this
//!   one is the shape, the node's is what the disk says.
//! - Cost: files whose names hold a backslash, a control character or
//!   NUL (legal on Unix) cannot be named; such names break on the other
//!   platforms and in every page anyway.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

/// A relative path inside a root, parts separated by `/`. The empty path is
/// the root itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct RelPath(Arc<str>);

/// Why a path was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidRelPath {
    /// It starts with `/` (absolute).
    Absolute,
    /// Its first part is a drive (`C:`).
    Drive,
    /// It holds a backslash, NUL or another control character.
    Character {
        /// The character.
        character: char,
    },
    /// An empty part (`a//b`, a trailing `/`), `.` or `..`.
    Part,
    /// A part named `.git`, in any case.
    GitDir,
    /// Longer than [`RelPath::MAX_BYTES`], or a part longer than
    /// [`RelPath::MAX_PART_BYTES`].
    TooLong,
}

impl fmt::Display for InvalidRelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absolute => f.write_str("a path here is relative to its folder: no leading `/`"),
            Self::Drive => f.write_str("a path here is relative to its folder: no drive"),
            Self::Character { character } => write!(f, "a path cannot contain {character:?}"),
            Self::Part => f.write_str("a path cannot have an empty, `.` or `..` part"),
            Self::GitDir => f.write_str("a path cannot name the repository's `.git`"),
            Self::TooLong => write!(
                f,
                "a path is at most {} bytes, each part at most {}",
                RelPath::MAX_BYTES,
                RelPath::MAX_PART_BYTES
            ),
        }
    }
}

impl std::error::Error for InvalidRelPath {}

impl RelPath {
    /// The longest path, in bytes.
    pub const MAX_BYTES: usize = 4096;
    /// The longest part, in bytes (what every common file system allows).
    pub const MAX_PART_BYTES: usize = 255;

    /// The root itself.
    #[must_use]
    pub fn root() -> Self {
        Self::default()
    }

    /// Whether this is the root itself.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// The path as text, parts separated by `/`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Its parts, first to last; none for the root.
    pub fn parts(&self) -> impl Iterator<Item = &str> {
        self.0.split('/').filter(|part| !part.is_empty())
    }

    /// Its last part; `None` for the root. (Not `name`: under the `ts`
    /// feature that is the name of the TypeScript type ts-rs derives.)
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        self.parts().last()
    }

    /// The path of its folder; the root for a top-level entry, `None` for
    /// the root.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        if self.is_root() {
            return None;
        }
        Some(
            self.0
                .rsplit_once('/')
                .map_or_else(Self::root, |(parent, _)| Self(Arc::from(parent))),
        )
    }

    /// `name` inside this folder.
    ///
    /// # Errors
    /// When `name` is not one valid part, or the result is too long.
    pub fn child(&self, name: &str) -> Result<Self, InvalidRelPath> {
        if name.contains('/') {
            return Err(InvalidRelPath::Part);
        }
        if self.is_root() {
            name.parse()
        } else {
            format!("{}/{name}", self.0).parse()
        }
    }

    /// Whether `self` is `other` or inside it.
    #[must_use]
    pub fn starts_with(&self, other: &Self) -> bool {
        other.is_root()
            || self.0 == other.0
            || self
                .0
                .strip_prefix(&*other.0)
                .is_some_and(|rest| rest.starts_with('/'))
    }

    /// The path below `root` on this machine, joined part by part (so a
    /// part is never read as a separator or a root by the platform).
    #[must_use]
    pub fn under(&self, root: &Path) -> PathBuf {
        let mut path = root.to_path_buf();
        for part in self.parts() {
            path.push(part);
        }
        path
    }
}

impl FromStr for RelPath {
    type Err = InvalidRelPath;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty() {
            return Ok(Self::root());
        }
        if text.len() > Self::MAX_BYTES {
            return Err(InvalidRelPath::TooLong);
        }
        if let Some(character) = text
            .chars()
            .find(|character| *character == '\\' || character.is_control())
        {
            return Err(InvalidRelPath::Character { character });
        }
        if text.starts_with('/') {
            return Err(InvalidRelPath::Absolute);
        }
        let mut parts = text.split('/').peekable();
        if parts.peek().is_some_and(|first| is_drive(first)) {
            return Err(InvalidRelPath::Drive);
        }
        for part in parts {
            if part.is_empty() || part == "." || part == ".." {
                return Err(InvalidRelPath::Part);
            }
            if is_git_dir(part) {
                return Err(InvalidRelPath::GitDir);
            }
            if part.len() > Self::MAX_PART_BYTES {
                return Err(InvalidRelPath::TooLong);
            }
        }
        Ok(Self(Arc::from(text)))
    }
}

/// Whether a part names a repository's `.git` on some platform, by git's own
/// rules (`verify_path` in git's `read-cache.c`): `.git` in any case; with
/// trailing dots or spaces, which Windows drops; `git~1`, its 8.3 short
/// name; and with the zero-width characters HFS+ ignores in names. Also
/// what a project's root may not contain (ADR 0030).
#[must_use]
pub fn is_git_dir(part: &str) -> bool {
    let visible: String = part
        .chars()
        .filter(|c| !matches!(c, '\u{200c}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{206a}'..='\u{206f}' | '\u{feff}'))
        .collect();
    let trimmed = visible.trim_end_matches(['.', ' ']);
    trimmed.eq_ignore_ascii_case(".git") || trimmed.eq_ignore_ascii_case("git~1")
}

/// `C:` or `C:anything`: a drive on Windows.
fn is_drive(part: &str) -> bool {
    let mut chars = part.chars();
    chars
        .next()
        .is_some_and(|letter| letter.is_ascii_alphabetic())
        && chars.next() == Some(':')
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

crate::key::serde_via_text!(RelPath);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_relative_paths_parse_and_join_part_by_part() {
        let path: RelPath = "src/main.rs".parse().unwrap();
        assert_eq!(path.parts().collect::<Vec<_>>(), ["src", "main.rs"]);
        assert_eq!(path.file_name(), Some("main.rs"));
        assert_eq!(path.parent().unwrap().as_str(), "src");
        assert_eq!(path.parent().unwrap().parent(), Some(RelPath::root()));
        assert_eq!(RelPath::root().parent(), None);
        let root = Path::new("root");
        assert_eq!(path.under(root), root.join("src").join("main.rs"));
        assert!(path.starts_with(&"src".parse().unwrap()));
        assert!(!path.starts_with(&"sr".parse().unwrap()));
        assert!(path.starts_with(&RelPath::root()));
        assert_eq!("".parse::<RelPath>(), Ok(RelPath::root()));
        assert!(".github/workflows".parse::<RelPath>().is_ok());
        assert!(
            "ab:c.txt".parse::<RelPath>().is_ok(),
            "a colon that is not a drive"
        );
        assert_eq!("a:b.txt".parse::<RelPath>(), Err(InvalidRelPath::Drive));
    }

    #[test]
    fn nothing_that_leaves_or_reaches_git_parses() {
        for (text, problem) in [
            ("/etc/passwd", InvalidRelPath::Absolute),
            ("C:", InvalidRelPath::Drive),
            ("c:/windows", InvalidRelPath::Drive),
            ("C:secret", InvalidRelPath::Drive),
            ("..\\..\\x", InvalidRelPath::Character { character: '\\' }),
            (
                "\\\\server\\share",
                InvalidRelPath::Character { character: '\\' },
            ),
            ("../x", InvalidRelPath::Part),
            ("a/../../x", InvalidRelPath::Part),
            ("a/./b", InvalidRelPath::Part),
            ("a//b", InvalidRelPath::Part),
            ("a/", InvalidRelPath::Part),
            (".git/config", InvalidRelPath::GitDir),
            ("sub/.GIT/hooks", InvalidRelPath::GitDir),
            (".git./config", InvalidRelPath::GitDir),
            (".git /config", InvalidRelPath::GitDir),
            ("GIT~1/config", InvalidRelPath::GitDir),
            (".g\u{200c}it/config", InvalidRelPath::GitDir),
            ("a\0b", InvalidRelPath::Character { character: '\0' }),
            ("a\nb", InvalidRelPath::Character { character: '\n' }),
        ] {
            assert_eq!(text.parse::<RelPath>(), Err(problem), "{text:?}");
        }
        assert_eq!(
            "a".repeat(256).parse::<RelPath>(),
            Err(InvalidRelPath::TooLong)
        );
        assert!(serde_json::from_str::<RelPath>(r#""../x""#).is_err());
    }

    #[test]
    fn a_child_is_one_part() {
        let dir: RelPath = "src".parse().unwrap();
        assert_eq!(dir.child("lib.rs").unwrap().as_str(), "src/lib.rs");
        assert_eq!(RelPath::root().child("a").unwrap().as_str(), "a");
        assert!(dir.child("a/b").is_err());
        assert!(dir.child("..").is_err());
    }
}
