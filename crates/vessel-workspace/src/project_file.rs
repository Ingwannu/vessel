//! A project's own file, `vessel.toml` at its root (ADR 0027): the commands
//! that set a new worktree up and tear it down.
//!
//! [Decision Log]
//! - Purpose: a repository says how a worktree of it is prepared (install
//!   dependencies, copy an environment file) and cleaned up, as Paseo's
//!   `paseo.json` (`worktree.setup`, `worktree.teardown`) and GOAT's
//!   `goat.json` (`scripts[].runOnWorktreeCreate`) do.
//! - Format: TOML, as every file of Vessel's a person edits
//!   (`profiles.toml`, the node book), with comments; `toml_edit` is
//!   already in the workspace. Paseo's shape (`setup`, `teardown`, each a
//!   string or a list) under a `[worktree]` table, so later tracks add
//!   `[scripts.*]`, `[services.*]` and `[env]` beside it without changing
//!   this one: every key this version does not read is ignored, not
//!   refused.
//! - The digest is what a person approves: SHA-256 over both lists in a
//!   length-prefixed encoding (so `["a b"]` and `["a", "b"]` differ). Any
//!   change to a command, its order, or which list it is in is another
//!   digest, asked again. A later track that runs more of the file adds
//!   what it runs to the digest, which makes every earlier approval ask
//!   again; that is the safe direction.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use vessel_proto::workspace::{CommandsDigest, InvalidText, ProjectCommand};

/// The file's name, at the project's root.
pub const FILE_NAME: &str = "vessel.toml";

/// The largest file read.
pub const MAX_BYTES: usize = 64 * 1024;

/// The most commands in one list.
pub const MAX_COMMANDS: usize = 32;

/// What the file says about worktrees.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectFile {
    /// Run in order when a worktree is created.
    pub setup: Vec<ProjectCommand>,
    /// Run in order before a worktree is removed.
    pub teardown: Vec<ProjectCommand>,
}

/// Why the file cannot be used. Nothing in it runs then.
#[derive(Debug, thiserror::Error)]
pub enum ProjectFileError {
    /// Larger than [`MAX_BYTES`].
    #[error("`vessel.toml` is larger than {MAX_BYTES} bytes")]
    TooLarge,
    /// Not UTF-8.
    #[error("`vessel.toml` is not UTF-8 text")]
    NotText,
    /// Not TOML, or `[worktree]` has the wrong shape.
    #[error("`vessel.toml` does not parse: {0}")]
    Syntax(String),
    /// A list longer than [`MAX_COMMANDS`].
    #[error("`worktree.{list}` names more than {MAX_COMMANDS} commands")]
    TooMany {
        /// `setup` or `teardown`.
        list: &'static str,
    },
    /// A command the rules refuse (empty, too long, NUL).
    #[error("`worktree.{list}`: {source}")]
    Command {
        /// `setup` or `teardown`.
        list: &'static str,
        /// What is wrong with it.
        source: InvalidText,
    },
}

#[derive(Deserialize, Default)]
struct Raw {
    #[serde(default)]
    worktree: RawWorktree,
}

#[derive(Deserialize, Default)]
struct RawWorktree {
    #[serde(default)]
    setup: Commands,
    #[serde(default)]
    teardown: Commands,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Commands {
    One(String),
    Many(Vec<String>),
}

impl Default for Commands {
    fn default() -> Self {
        Self::Many(Vec::new())
    }
}

impl ProjectFile {
    /// Parse the file's bytes.
    ///
    /// # Errors
    /// As [`ProjectFileError`] says.
    pub fn parse(bytes: &[u8]) -> Result<Self, ProjectFileError> {
        if bytes.len() > MAX_BYTES {
            return Err(ProjectFileError::TooLarge);
        }
        let text = std::str::from_utf8(bytes).map_err(|_not_utf8| ProjectFileError::NotText)?;
        let raw: Raw = toml_edit::de::from_str(text)
            .map_err(|error| ProjectFileError::Syntax(error.message().to_owned()))?;
        Ok(Self {
            setup: commands("setup", raw.worktree.setup)?,
            teardown: commands("teardown", raw.worktree.teardown)?,
        })
    }

    /// Whether it names no command.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.setup.is_empty() && self.teardown.is_empty()
    }

    /// What a person approves: SHA-256 over both lists.
    #[must_use]
    pub fn digest(&self) -> CommandsDigest {
        let mut hash = Sha256::new();
        hash.update(b"vessel.toml worktree commands 1\n");
        for (name, list) in [("setup", &self.setup), ("teardown", &self.teardown)] {
            hash.update(name.as_bytes());
            hash.update(u64::try_from(list.len()).unwrap_or(u64::MAX).to_le_bytes());
            for command in list {
                let bytes = command.as_str().as_bytes();
                hash.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
                hash.update(bytes);
            }
        }
        CommandsDigest::from_sha256(hash.finalize().into())
    }
}

fn commands(list: &'static str, raw: Commands) -> Result<Vec<ProjectCommand>, ProjectFileError> {
    let texts = match raw {
        Commands::One(text) => vec![text],
        Commands::Many(texts) => texts,
    };
    if texts.len() > MAX_COMMANDS {
        return Err(ProjectFileError::TooMany { list });
    }
    texts
        .iter()
        .map(|text| {
            text.parse()
                .map_err(|source| ProjectFileError::Command { list, source })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_lists_and_other_tables_parse() {
        let file = ProjectFile::parse(
            br#"
            # comments are fine
            [worktree]
            setup = ["npm ci", "cp \"$VESSEL_PROJECT_ROOT/.env\" ."]
            teardown = "docker compose down"

            [scripts.dev]
            command = "npm run dev"
            "#,
        )
        .unwrap();
        assert_eq!(file.setup.len(), 2);
        assert_eq!(
            file.teardown.first().unwrap().as_str(),
            "docker compose down"
        );
        assert!(ProjectFile::parse(b"").unwrap().is_empty());
        assert!(
            ProjectFile::parse(b"[scripts.x]\ncommand = 'y'")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_bad_file_runs_nothing() {
        assert!(matches!(
            ProjectFile::parse(b"[worktree\nsetup = 1"),
            Err(ProjectFileError::Syntax(_))
        ));
        assert!(matches!(
            ProjectFile::parse(b"[worktree]\nsetup = 1"),
            Err(ProjectFileError::Syntax(_))
        ));
        assert!(matches!(
            ProjectFile::parse(b"[worktree]\nsetup = ['  ']"),
            Err(ProjectFileError::Command { list: "setup", .. })
        ));
        let many = format!("[worktree]\nteardown = [{}]", vec!["'x'"; 33].join(","));
        assert!(matches!(
            ProjectFile::parse(many.as_bytes()),
            Err(ProjectFileError::TooMany { list: "teardown" })
        ));
        assert!(matches!(
            ProjectFile::parse(&vec![b'#'; MAX_BYTES + 1]),
            Err(ProjectFileError::TooLarge)
        ));
        assert!(matches!(
            ProjectFile::parse(&[0xff, 0xfe]),
            Err(ProjectFileError::NotText)
        ));
    }

    #[test]
    fn any_change_to_what_runs_is_another_digest() {
        let digest = |text: &str| ProjectFile::parse(text.as_bytes()).unwrap().digest();
        let base = digest("[worktree]\nsetup = ['a b']");
        assert_eq!(base, digest("[worktree]\nsetup = 'a b' # same"));
        assert_ne!(base, digest("[worktree]\nsetup = ['a', 'b']"));
        assert_ne!(base, digest("[worktree]\nteardown = ['a b']"));
        assert_ne!(base, digest("[worktree]\nsetup = ['a b ']"));
        assert_eq!(
            digest("[worktree]\nsetup = ['x']\n[env]\nA = '1'"),
            digest("[worktree]\nsetup = ['x']"),
            "keys this version does not run are not approved here"
        );
    }
}
