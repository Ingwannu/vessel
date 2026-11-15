//! Bounded project styles for generated text (ADR 0062). This decoding
//! edge deliberately tolerates malformed siblings independently; it
//! neither runs commands nor contributes text to their approval digest.

use std::path::Path;
use std::sync::Arc;

use super::{MAX_BYTES, ProjectFileError};
use serde::Deserialize;

/// The raw nonblank instruction bound, in Unicode scalar values.
pub const MAX_STYLE_CHARS: usize = 1024;

/// Nonempty, trimmed instructions whose bound keeps the model input small.
#[derive(Clone, PartialEq, Eq)]
pub struct StyleText(Arc<str>);

// Project instructions become prompt content and must not enter a log
// merely because an enclosing request derives Debug.
impl std::fmt::Debug for StyleText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StyleText(..)")
    }
}

impl StyleText {
    fn parse(text: &str) -> Result<Option<Self>, StyleError> {
        if text.trim().is_empty() {
            return Ok(None);
        }
        if text.chars().count() > MAX_STYLE_CHARS {
            return Err(StyleError::TooLong);
        }
        let trimmed = Arc::from(text.trim());
        Ok(Some(Self(trimmed)))
    }

    /// Borrow the checked instructions for the prompt's style section.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
enum StyleError {
    #[error("instructions exceed {MAX_STYLE_CHARS} characters")]
    TooLong,
}

/// Per-kind instructions; absence selects that kind's built-in style.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectStyles {
    /// `[metadata.title]`, replacing the title style only.
    pub title: Option<StyleText>,
    /// `[metadata.branch]`, independent of the title.
    pub branch: Option<StyleText>,
    /// `[metadata.commit]`, leaving the output contract fixed.
    pub commit: Option<StyleText>,
    /// `[metadata.pull]`, shared by its title and body.
    pub pull: Option<StyleText>,
}

impl ProjectStyles {
    /// Blocking: read at an already resolved repository root (or the working folder
    /// outside a repository). The existing project-file reader enforces
    /// containment, regular-file and byte bounds before parsing. Errors
    /// select defaults and log classes without any project text.
    #[must_use]
    pub fn read(folder: &Path) -> Self {
        match super::read_bytes(folder) {
            Ok(Some(bytes)) => styles_in(&bytes),
            Ok(None) => Self::default(),
            Err(error) => {
                tracing::warn!(
                    class = class_of(&error).label(),
                    "project metadata was not used"
                );
                Self::default()
            }
        }
    }
}

#[derive(Default, Deserialize)]
struct RawFile {
    #[serde(default)]
    metadata: RawMetadata,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawMetadata {
    Styles(RawStyles),
    Malformed(serde::de::IgnoredAny),
}

impl Default for RawMetadata {
    fn default() -> Self {
        Self::Styles(RawStyles::default())
    }
}

#[derive(Default, Deserialize)]
struct RawStyles {
    #[serde(default)]
    title: RawEntry,
    #[serde(default)]
    branch: RawEntry,
    #[serde(default)]
    commit: RawEntry,
    #[serde(default)]
    pull: RawEntry,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawEntry {
    Table {
        instructions: Option<RawInstructions>,
    },
    Malformed(serde::de::IgnoredAny),
}

impl Default for RawEntry {
    fn default() -> Self {
        Self::Table { instructions: None }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawInstructions {
    Text(String),
    Malformed(serde::de::IgnoredAny),
}

#[derive(Clone, Copy)]
enum Kind {
    Title,
    Branch,
    Commit,
    Pull,
}

impl Kind {
    const fn label(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Branch => "branch",
            Self::Commit => "commit",
            Self::Pull => "pull",
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum StyleFileError {
    #[error("metadata file exceeds {MAX_BYTES} bytes")]
    TooLarge,
    #[error("metadata file is not UTF-8")]
    NotText(#[source] std::str::Utf8Error),
    #[error("metadata file is not TOML")]
    Syntax(#[source] toml_edit::de::Error),
}

impl StyleFileError {
    fn class(&self) -> Problem {
        match self {
            Self::TooLarge => Problem::TooLarge,
            Self::NotText(_) => Problem::NotText,
            Self::Syntax(_) => Problem::Syntax,
        }
    }
}

fn styles_in(bytes: &[u8]) -> ProjectStyles {
    match parse_styles(bytes) {
        Ok(styles) => styles,
        Err(error) => {
            // A parser diagnostic may quote project text. Classify only
            // here, after retaining its typed source through the decoder.
            tracing::warn!(
                class = error.class().label(),
                "project metadata was not used"
            );
            ProjectStyles::default()
        }
    }
}

fn parse_styles(bytes: &[u8]) -> Result<ProjectStyles, StyleFileError> {
    if bytes.len() > MAX_BYTES {
        return Err(StyleFileError::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(StyleFileError::NotText)?;
    let file = toml_edit::de::from_str::<RawFile>(text).map_err(StyleFileError::Syntax)?;
    match file.metadata {
        RawMetadata::Styles(raw) => Ok(ProjectStyles {
            title: one(raw.title, Kind::Title),
            branch: one(raw.branch, Kind::Branch),
            commit: one(raw.commit, Kind::Commit),
            pull: one(raw.pull, Kind::Pull),
        }),
        RawMetadata::Malformed(_) => {
            tracing::warn!(
                class = Problem::Syntax.label(),
                "project metadata was not used"
            );
            Ok(ProjectStyles::default())
        }
    }
}

#[derive(Clone, Copy)]
enum Problem {
    TooLong,
    NotText,
    Syntax,
    TooLarge,
    Unreadable,
}

impl Problem {
    const fn label(self) -> &'static str {
        match self {
            Self::TooLong => "too_long",
            Self::NotText => "not_text",
            Self::Syntax => "syntax",
            Self::TooLarge => "too_large",
            Self::Unreadable => "unreadable",
        }
    }
}

fn one(raw: RawEntry, kind: Kind) -> Option<StyleText> {
    match raw {
        RawEntry::Table { instructions: None } => None,
        RawEntry::Table {
            instructions: Some(RawInstructions::Text(text)),
        } => match StyleText::parse(&text) {
            Ok(style) => style,
            Err(StyleError::TooLong) => {
                refused(kind, Problem::TooLong);
                None
            }
        },
        RawEntry::Table {
            instructions: Some(RawInstructions::Malformed(_)),
        } => {
            refused(kind, Problem::NotText);
            None
        }
        RawEntry::Malformed(_) => {
            refused(kind, Problem::Syntax);
            None
        }
    }
}

fn refused(kind: Kind, problem: Problem) {
    tracing::warn!(
        table = kind.label(),
        class = problem.label(),
        "metadata instructions were not used"
    );
}

fn class_of(error: &ProjectFileError) -> Problem {
    match error {
        ProjectFileError::TooLarge => Problem::TooLarge,
        ProjectFileError::NotText => Problem::NotText,
        ProjectFileError::Syntax(_)
        | ProjectFileError::TooMany { .. }
        | ProjectFileError::Command { .. }
        | ProjectFileError::Scripts(_)
        | ProjectFileError::TerminalMissingCommand
        | ProjectFileError::TerminalCommand(_)
        | ProjectFileError::TerminalName(_)
        | ProjectFileError::TooManyTerminals => Problem::Syntax,
        ProjectFileError::NotAFile | ProjectFileError::Unreadable(_) => Problem::Unreadable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_file::ProjectFile;

    #[test]
    fn styles_work_in_tables_and_inline_tables_and_are_independent() {
        let tables = b"[metadata.title]\ninstructions = 'Use sentence case.'\n[metadata.branch]\ninstructions = '   '\n";
        let inline = b"metadata = { title = { instructions = 'Use sentence case.' }, branch = { instructions = '   ' } }\n";
        let expected = styles_in(tables);
        assert_eq!(styles_in(inline), expected);
        assert_eq!(expected.title.unwrap().as_str(), "Use sentence case.");
        assert!(expected.branch.is_none());
    }

    #[test]
    fn invalid_siblings_do_not_remove_the_valid_one() {
        let styles = styles_in(b"agentTitle = 'ignored'\n[metadata]\ntitle = 1\nbranch = { instructions = 3 }\ncommit = { instructions = 'Keep identifiers.' }\n");
        assert!(styles.title.is_none());
        assert!(styles.branch.is_none());
        assert_eq!(styles.commit.unwrap().as_str(), "Keep identifiers.");
        for input in [b"metadata = [".as_slice(), b"metadata = 1", b"\xff"] {
            assert_eq!(styles_in(input), ProjectStyles::default());
        }
    }

    #[test]
    fn unicode_character_bound_is_exact_and_not_a_truncation() {
        let exact = "한".repeat(MAX_STYLE_CHARS);
        let input = format!("[metadata.title]\ninstructions = '{exact}'\n");
        assert_eq!(styles_in(input.as_bytes()).title.unwrap().as_str(), exact);
        let long = format!(
            "[metadata.title]\ninstructions = '{exact}글'\n[metadata.pull]\ninstructions = 'Brief.'\n"
        );
        let styles = styles_in(long.as_bytes());
        assert!(styles.title.is_none());
        assert_eq!(styles.pull.unwrap().as_str(), "Brief.");
    }

    #[test]
    fn styles_do_not_change_command_parsing_or_approval() {
        let commands = b"[worktree]\nsetup = 'echo hello'\n";
        let with_style =
            b"[worktree]\nsetup = 'echo hello'\n[metadata.title]\ninstructions = 'Brief.'\n";
        assert_eq!(
            ProjectFile::parse(commands).unwrap().digest(),
            ProjectFile::parse(with_style).unwrap().digest()
        );
    }

    #[test]
    fn padding_does_not_repair_the_raw_bound_and_debug_omits_instructions() {
        let exact = "x".repeat(MAX_STYLE_CHARS);
        assert!(matches!(
            StyleText::parse(&format!(" {exact} ")),
            Err(StyleError::TooLong)
        ));
        assert!(
            StyleText::parse(&" ".repeat(MAX_STYLE_CHARS.saturating_add(1)))
                .unwrap()
                .is_none()
        );
        let styles =
            styles_in(b"[metadata.title]\ninstructions = 'distinctive project instruction'\n");
        assert!(!format!("{styles:?}").contains("distinctive project instruction"));
        assert_eq!(
            styles.title.unwrap().as_str(),
            "distinctive project instruction"
        );
    }

    #[test]
    fn malformed_commands_do_not_block_a_valid_style() {
        let file = b"[worktree]\nsetup = 7\n[metadata.title]\ninstructions = 'Brief.'\n";
        assert!(ProjectFile::parse(file).is_err());
        assert_eq!(styles_in(file).title.unwrap().as_str(), "Brief.");
    }

    #[test]
    fn disk_reader_uses_the_resolved_folder_and_missing_means_default() {
        let folder = tempfile::tempdir().unwrap();
        assert_eq!(ProjectStyles::read(folder.path()), ProjectStyles::default());
        std::fs::write(
            folder.path().join("vessel.toml"),
            "[metadata.title]\ninstructions = 'Brief.'\n",
        )
        .unwrap();
        assert_eq!(
            ProjectStyles::read(folder.path()).title.unwrap().as_str(),
            "Brief."
        );
        std::fs::write(
            folder.path().join("vessel.toml"),
            vec![b'x'; MAX_BYTES.saturating_add(1)],
        )
        .unwrap();
        assert_eq!(ProjectStyles::read(folder.path()), ProjectStyles::default());
    }
}
