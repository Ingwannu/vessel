//! The bounded strings of the workspace calls (protocol 10): git names,
//! commit ids and messages, a project's name and commands, queries.
//!
//! [Decision Log]
//! - Purpose: each of these ends up as an argument to `git` or a shell, a
//!   folder name, or a line on a page. A branch name starting with `-`
//!   is an option to git; an unbounded query is a denial of service.
//! - Constraint: AGENTS.md section 2: a value with a shape is a type whose
//!   only constructor checks it, so the node, the store and every client
//!   hold only checked values.
//! - Alternatives: `git check-ref-format` run by the node (a process per
//!   check, and a client could not check what it sends); one `GitName`
//!   type for branches, refs and remotes (a remote could then be passed
//!   where a branch is wanted).
//! - Choice: one rule function per kind ([`TextKind`]), one error
//!   ([`InvalidText`]), and a private macro declaring each newtype from its
//!   kind, decoded through the parse (`serde_via_text`).
//! - The git rules are `git check-ref-format`'s (git's documentation,
//!   "check-ref-format"), applied to the whole name, plus one of ours: no
//!   leading `-`, which git allows for refs but would read as an option.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// Which bounded string, and so which rule applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextKind {
    /// A branch name: git's ref rules, not `HEAD`.
    Branch,
    /// A branch, a remote-tracking branch (`origin/main`) or a tag.
    Ref,
    /// A remote's name: one component of the ref rules.
    Remote,
    /// A commit id: 7 to 64 hexadecimal digits.
    Commit,
    /// A Git-observed unique display abbreviation: 4 to 64 hexadecimal digits.
    CommitAbbreviation,
    /// A commit message: not blank, at most 64 KiB, no NUL.
    CommitMessage,
    /// A project's display name: 1 to 80 characters, one line.
    ProjectName,
    /// One command from a project's file: not blank, at most 4 KiB, no NUL.
    Command,
    /// A digest of a project's commands: 64 hexadecimal digits.
    Digest,
    /// A file's content hash: 64 hexadecimal digits.
    Hash,
    /// One folder to create: the bundle path rules for one segment.
    Folder,
    /// A file name query: 1 to 256 characters, one line.
    FindQuery,
    /// A search in files: 1 to 1,000 characters, one line.
    GrepQuery,
    /// A git configuration key named in a refusal (protocol 11, ADR
    /// 0030): 1 to 256 characters, one line.
    ConfigKey,
}

impl fmt::Display for TextKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Branch => "branch name",
            Self::Ref => "ref name",
            Self::Remote => "remote name",
            Self::Commit => "commit id",
            Self::CommitAbbreviation => "commit abbreviation",
            Self::CommitMessage => "commit message",
            Self::ProjectName => "project name",
            Self::Command => "project command",
            Self::Digest => "commands digest",
            Self::Hash => "content hash",
            Self::Folder => "folder name",
            Self::FindQuery => "file name query",
            Self::GrepQuery => "search query",
            Self::ConfigKey => "configuration key",
        })
    }
}

/// What is wrong with a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextProblem {
    /// Empty, or only white space.
    Empty,
    /// Longer than the kind allows (`max` in the kind's unit: bytes for
    /// git names, commands and messages, characters for the rest).
    TooLong {
        /// The bound.
        max: usize,
    },
    /// A character the kind does not allow.
    Character {
        /// The character.
        character: char,
    },
    /// A sequence or a shape the kind does not allow, described.
    Shape {
        /// What the rule says.
        rule: &'static str,
    },
}

/// A string refused for its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidText {
    /// What it was meant to be.
    pub kind: TextKind,
    /// What is wrong.
    pub problem: TextProblem,
}

impl fmt::Display for InvalidText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = self.kind;
        match &self.problem {
            TextProblem::Empty => write!(f, "a {kind} cannot be empty"),
            TextProblem::TooLong { max } => write!(f, "a {kind} is at most {max} long"),
            TextProblem::Character { character } => {
                write!(f, "a {kind} cannot contain {character:?}")
            }
            TextProblem::Shape { rule } => write!(f, "a {kind} {rule}"),
        }
    }
}

impl std::error::Error for InvalidText {}

impl TextKind {
    /// Check `text` against this kind's rule; the text to keep (trimmed
    /// for display names, lower case for hexadecimal ids, as given
    /// otherwise).
    fn parse(self, text: &str) -> Result<Arc<str>, TextProblem> {
        match self {
            Self::Branch => {
                ref_rule(text, true)?;
                if text == "HEAD" {
                    return Err(TextProblem::Shape {
                        rule: "cannot be `HEAD`",
                    });
                }
                Ok(Arc::from(text))
            }
            Self::Ref => ref_rule(text, true).map(|()| Arc::from(text)),
            Self::Remote => ref_rule(text, false).map(|()| Arc::from(text)),
            Self::Commit => hex(text, 7, 64),
            Self::CommitAbbreviation => hex(text, 4, 64),
            Self::Digest | Self::Hash => hex(text, 64, 64),
            Self::Folder => folder(text).map(|()| Arc::from(text)),
            Self::CommitMessage => free_text(text, 64 * 1024),
            Self::Command => free_text(text, 4 * 1024),
            Self::ProjectName => line(text.trim(), 80).map(|()| Arc::from(text.trim())),
            Self::FindQuery => line(text, 256).map(|()| Arc::from(text)),
            Self::GrepQuery => line(text, 1000).map(|()| Arc::from(text)),
            Self::ConfigKey => line(text, 256).map(|()| Arc::from(text)),
        }
    }
}

/// git's `check-ref-format` rules, and no leading `-`. `slashes` allows
/// more than one component (a remote's name is one).
fn ref_rule(text: &str, slashes: bool) -> Result<(), TextProblem> {
    const MAX: usize = 255;
    let shape = |rule| Err(TextProblem::Shape { rule });
    if text.is_empty() {
        return Err(TextProblem::Empty);
    }
    if text.len() > MAX {
        return Err(TextProblem::TooLong { max: MAX });
    }
    if let Some(character) = text.chars().find(|character| {
        character.is_ascii_control()
            || matches!(character, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
    }) {
        return Err(TextProblem::Character { character });
    }
    if text.starts_with('-') {
        return shape("cannot start with `-`");
    }
    if text.contains("..") {
        return shape("cannot contain `..`");
    }
    if text.contains("@{") || text == "@" {
        return shape("cannot contain `@{` or be `@`");
    }
    if text.ends_with('.') {
        return shape("cannot end with `.`");
    }
    if !slashes && text.contains('/') {
        return Err(TextProblem::Character { character: '/' });
    }
    for component in text.split('/') {
        if component.is_empty() {
            return shape("cannot start or end with `/` or contain `//`");
        }
        if component.starts_with('.') {
            return shape("cannot have a part starting with `.`");
        }
        // git's rule is case-sensitive: `A.LOCK` is a valid name.
        if component.as_bytes().ends_with(b".lock") {
            return shape("cannot have a part ending with `.lock`");
        }
    }
    Ok(())
}

/// One folder to create on any platform: the bundle path rules for a
/// segment (no `\\ : * ? " < > |`, no control character, not `.` or `..`,
/// no trailing dot or space, no Windows device or short name), no `/`, no
/// leading `-`, at most 255 bytes.
fn folder(text: &str) -> Result<(), TextProblem> {
    const MAX: usize = 255;
    if text.is_empty() {
        return Err(TextProblem::Empty);
    }
    if text.len() > MAX {
        return Err(TextProblem::TooLong { max: MAX });
    }
    if text.contains('/') {
        return Err(TextProblem::Character { character: '/' });
    }
    if text.starts_with('-') {
        return Err(TextProblem::Shape {
            rule: "cannot start with `-`",
        });
    }
    crate::fleet::check_segment(text).map_err(|_rule| TextProblem::Shape {
        rule: "is one folder name every platform can create",
    })
}

/// `min` to `max` hexadecimal digits, kept in lower case.
fn hex(text: &str, min: usize, max: usize) -> Result<Arc<str>, TextProblem> {
    if text.is_empty() {
        return Err(TextProblem::Empty);
    }
    if let Some(character) = text
        .chars()
        .find(|character| !character.is_ascii_hexdigit())
    {
        return Err(TextProblem::Character { character });
    }
    if text.len() > max {
        return Err(TextProblem::TooLong { max });
    }
    if text.len() < min {
        return Err(TextProblem::Shape {
            rule: "has too few digits",
        });
    }
    Ok(Arc::from(text.to_ascii_lowercase()))
}

/// Free text: not blank, at most `max` bytes, no NUL; kept as given.
fn free_text(text: &str, max: usize) -> Result<Arc<str>, TextProblem> {
    if text.trim().is_empty() {
        return Err(TextProblem::Empty);
    }
    if text.len() > max {
        return Err(TextProblem::TooLong { max });
    }
    if text.contains('\0') {
        return Err(TextProblem::Character { character: '\0' });
    }
    Ok(Arc::from(text))
}

/// One line: not blank, at most `max` characters, no control character.
fn line(text: &str, max: usize) -> Result<(), TextProblem> {
    if text.trim().is_empty() {
        return Err(TextProblem::Empty);
    }
    if let Some(character) = text.chars().find(|character| character.is_control()) {
        return Err(TextProblem::Character { character });
    }
    if text.chars().count() > max {
        return Err(TextProblem::TooLong { max });
    }
    Ok(())
}

/// Declares a bounded string of one [`TextKind`].
macro_rules! bounded_text {
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
            type Err = InvalidText;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                TextKind::$kind
                    .parse(text)
                    .map(Self)
                    .map_err(|problem| InvalidText {
                        kind: TextKind::$kind,
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

bounded_text! {
    /// A git branch name (`refs/heads/<name>`).
    BranchName = Branch;
    /// A name git resolves: a branch, a remote-tracking branch such as
    /// `origin/main`, or a tag.
    RefName = Ref;
    /// A remote's name (`origin`).
    RemoteName = Remote;
    /// A commit's id, in full or abbreviated, lower case.
    CommitId = Commit;
    /// Git’s unique display abbreviation; never used as request identity.
    CommitAbbreviation = CommitAbbreviation;
    /// A commit's message.
    CommitMessage = CommitMessage;
    /// What a project is called on the page.
    ProjectName = ProjectName;
    /// One shell command from a project's file (`vessel.toml`).
    ProjectCommand = Command;
    /// SHA-256 of a project's setup and teardown commands, hexadecimal: what
    /// a person approves (ADR 0027).
    CommandsDigest = Digest;
    /// SHA-256 of a file's content, hexadecimal.
    ContentHash = Hash;
    /// The name of one folder to create (a clone's).
    FolderName = Folder;
    /// What `file.find` looks for.
    FindQuery = FindQuery;
    /// What `file.grep` looks for.
    GrepQuery = GrepQuery;
    /// A key of a repository's git configuration (`core.sshcommand`), named
    /// when a run is refused for it (protocol 11, ADR 0030); never its
    /// value.
    ConfigKey = ConfigKey;
}

impl CommandsDigest {
    /// A SHA-256 hash as a digest.
    #[must_use]
    pub fn from_sha256(hash: [u8; 32]) -> Self {
        Self(sha256_hex(hash))
    }
}

impl ContentHash {
    /// A SHA-256 hash of a file's content.
    #[must_use]
    pub fn from_sha256(hash: [u8; 32]) -> Self {
        Self(sha256_hex(hash))
    }
}

/// 32 bytes as 64 lowercase hexadecimal digits.
fn sha256_hex(hash: [u8; 32]) -> Arc<str> {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut hex = String::with_capacity(64);
    for byte in hash {
        for nibble in [byte >> 4, byte & 0x0f] {
            hex.push(char::from(
                DIGITS.get(usize::from(nibble)).copied().unwrap_or(b'0'),
            ));
        }
    }
    Arc::from(hex)
}

/// A client's number for a search, so a newer one with the same number
/// replaces it (`file.find`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct SearchKey(pub u32);

impl BranchName {
    /// The same name as a [`RefName`], for a call that takes any ref.
    #[must_use]
    pub fn as_ref_name(&self) -> RefName {
        RefName(Arc::clone(&self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused<T: FromStr<Err = InvalidText>>(text: &str) -> TextProblem {
        match text.parse::<T>() {
            Ok(_) => panic!("{text:?} was accepted"),
            Err(error) => error.problem,
        }
    }

    #[test]
    fn branch_names_follow_check_ref_format_and_never_start_with_a_dash() {
        for good in ["main", "feature/login", "vessel/3f9a2c1b", "a.b", "v1.2-rc"] {
            assert!(good.parse::<BranchName>().is_ok(), "{good}");
        }
        for bad in [
            "",
            "-rf",
            "--force",
            "a..b",
            "a b",
            "a~1",
            "a^",
            "a:b",
            "a?",
            "a*",
            "a[",
            "a\\b",
            "/a",
            "a/",
            "a//b",
            ".a",
            "a/.b",
            "a.lock",
            "a/b.lock/c",
            "a.",
            "@",
            "a@{1}",
            "HEAD",
            "tab\there",
        ] {
            assert!(bad.parse::<BranchName>().is_err(), "{bad:?} was accepted");
        }
        assert!(
            "a".repeat(256).parse::<BranchName>().is_err(),
            "too long is refused"
        );
        assert_eq!(
            refused::<BranchName>("-x"),
            TextProblem::Shape {
                rule: "cannot start with `-`"
            }
        );
    }

    #[test]
    fn refs_take_remote_branches_and_head_remotes_take_one_part() {
        assert!("origin/main".parse::<RefName>().is_ok());
        assert!("HEAD".parse::<RefName>().is_ok());
        assert!("origin".parse::<RemoteName>().is_ok());
        assert_eq!(
            refused::<RemoteName>("a/b"),
            TextProblem::Character { character: '/' }
        );
        assert!("-o".parse::<RemoteName>().is_err());
    }

    #[test]
    fn commit_ids_are_hex_and_kept_in_lower_case() {
        let id: CommitId = "ABCDEF1".parse().unwrap();
        assert_eq!(id.as_str(), "abcdef1");
        assert!("abc".parse::<CommitId>().is_err(), "too short");
        assert_eq!(
            "ABCD".parse::<CommitAbbreviation>().unwrap().as_str(),
            "abcd"
        );
        assert!("abc".parse::<CommitAbbreviation>().is_err());
        assert!("a".repeat(65).parse::<CommitAbbreviation>().is_err());
        assert!("g123456".parse::<CommitId>().is_err());
        assert!("a".repeat(65).parse::<CommitId>().is_err());
        assert!("a".repeat(64).parse::<CommandsDigest>().is_ok());
        assert!("a".repeat(63).parse::<CommandsDigest>().is_err());
        let digest = CommandsDigest::from_sha256([0xab; 32]);
        assert_eq!(digest.as_str(), "ab".repeat(32));
        assert_eq!(digest.as_str().parse::<CommandsDigest>(), Ok(digest));
    }

    #[test]
    fn free_text_and_lines() {
        assert!("fix: one\n\nbody".parse::<CommitMessage>().is_ok());
        assert_eq!(refused::<CommitMessage>("  \n"), TextProblem::Empty);
        assert!("a\0b".parse::<CommitMessage>().is_err());
        assert!(
            "x".repeat(64 * 1024 + 1).parse::<CommitMessage>().is_err(),
            "over 64 KiB"
        );
        let name: ProjectName = "  vessel  ".parse().unwrap();
        assert_eq!(name.as_str(), "vessel");
        assert!("two\nlines".parse::<ProjectName>().is_err());
        assert!("가".repeat(80).parse::<ProjectName>().is_ok(), "characters");
        assert!("npm ci && npm test".parse::<ProjectCommand>().is_ok());
        assert!("x".repeat(4097).parse::<ProjectCommand>().is_err());
        assert!("main.rs".parse::<FindQuery>().is_ok());
        assert!("a\nb".parse::<GrepQuery>().is_err());
        for good in ["repo", "my-app", "app.v2", "가나다"] {
            assert!(good.parse::<FolderName>().is_ok(), "{good}");
        }
        for bad in [
            "", ".", "..", "a/b", "a\\b", "C:x", "-x", "CON", "nul.txt", "PROGRA~1", "a.", "a ",
        ] {
            assert!(bad.parse::<FolderName>().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_bad_name_does_not_decode() {
        assert!(serde_json::from_str::<BranchName>(r#""-delete""#).is_err());
        let branch: BranchName = serde_json::from_str(r#""topic""#).unwrap();
        assert_eq!(serde_json::to_string(&branch).unwrap(), r#""topic""#);
        assert_eq!(branch.as_ref_name().as_str(), "topic");
    }
}
