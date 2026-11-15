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
//! - Protocol 11 (ADR 0031) runs `[scripts.*]` and `[services.*]` too
//!   (`scripts`): a file that names one is digested as version 2, which
//!   covers them; a file that names none keeps version 1 exactly, so an
//!   approval of it still covers what it showed.

use std::io::Read as _;
use std::path::Path;

/// Bounded prompt styles kept separate from executable project commands.
pub mod metadata;
mod scripts;
mod terminals;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use vessel_proto::scripts::{ScriptEntry, ScriptName};
use vessel_proto::workspace::{
    CommandsDigest, FileTerminal, InvalidText, ProjectCommand, RelPath, ServicePorts,
};

pub use scripts::{MAX_SCRIPTS, ScriptsError};

use crate::paths::{PathError, Root};

/// The file's name, at the project's root.
pub const FILE_NAME: &str = "vessel.toml";

/// The largest file read.
pub const MAX_BYTES: usize = 64 * 1024;

/// The shared command-list admission limit, retained under the parser API name.
pub use vessel_proto::workspace::MAX_PROJECT_COMMANDS as MAX_COMMANDS;

/// What the file says about worktrees, and its scripts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectFile {
    /// Run in order when a worktree is created.
    pub setup: Vec<ProjectCommand>,
    /// Run in order before a worktree is removed.
    pub teardown: Vec<ProjectCommand>,
    /// Its scripts and services, in name order (protocol 11).
    pub scripts: Vec<ScriptEntry>,
    /// Approved shell input for terminals opened after setup (ADR 0066).
    pub terminals: Vec<FileTerminal>,
    /// Preserve the approved allocation block even without configured services.
    pub service_ports: Option<ServicePorts>,
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
    /// A script or a service the rules refuse (protocol 11).
    #[error(transparent)]
    Scripts(#[from] ScriptsError),
    /// A terminal entry has no command to approve.
    #[error("`worktree.terminals` entry has no command")]
    TerminalMissingCommand,
    /// A terminal command must obey the same limits as setup commands.
    #[error("`worktree.terminals.command`: {0}")]
    TerminalCommand(#[source] InvalidText),
    /// A terminal name must fit the title the client can display.
    #[error("`worktree.terminals.name`: {0}")]
    TerminalName(#[source] vessel_proto::InvalidTitle),
    /// Refuse the whole file rather than silently dropping excess entries.
    #[error("`worktree.terminals` names more than {MAX_COMMANDS} terminals")]
    TooManyTerminals,
    /// Not a regular file (a device, a pipe, a folder).
    #[error("`vessel.toml` is not a regular file; it is not read")]
    NotAFile,
    /// The disk refused.
    #[error("`vessel.toml` cannot be read")]
    Unreadable(#[source] std::io::Error),
}

#[derive(Deserialize, Default)]
struct Raw {
    #[serde(default)]
    worktree: RawWorktree,
    #[serde(default)]
    scripts: scripts::RawTable,
    #[serde(default)]
    services: scripts::RawTable,
}

#[derive(Deserialize, Default)]
struct RawWorktree {
    #[serde(default)]
    setup: Commands,
    #[serde(default)]
    teardown: Commands,
    #[serde(default)]
    terminals: Vec<terminals::RawTerminal>,
    #[serde(default)]
    service_ports: Option<ServicePorts>,
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
            scripts: scripts::entries(raw.scripts, raw.services)?,
            terminals: terminals::parse(raw.worktree.terminals)?,
            service_ports: raw.worktree.service_ports,
        })
    }

    /// The file in `folder` as it is: `None` when there is none, or when
    /// it leads out of the folder.
    ///
    /// Blocking. Read only when it is a regular file inside `folder` once
    /// links are resolved, and never more than one byte past
    /// [`MAX_BYTES`]: anyone who can write in a project's folder (an agent
    /// working in it) could otherwise link it to a file of the node's,
    /// whose text a parse error might show, or to a device that never ends
    /// (`/dev/zero`), whose read would fill the node's memory. A same-user
    /// process can still swap the file between the check and the read
    /// (ADR 0027's limit for every path).
    ///
    /// # Errors
    /// As [`ProjectFileError`] says.
    pub fn read_in(folder: &Path) -> Result<Option<Self>, ProjectFileError> {
        read_bytes(folder)?
            .map(|bytes| Self::parse(&bytes))
            .transpose()
    }

    /// Whether nothing in the file requires approval.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.setup.is_empty()
            && self.teardown.is_empty()
            && self.scripts.is_empty()
            && self.terminals.is_empty()
            && self.service_ports.is_none()
    }

    /// Its script or service called `name`.
    #[must_use]
    pub fn script(&self, name: &ScriptName) -> Option<&ScriptEntry> {
        self.scripts.iter().find(|script| &script.name == name)
    }

    /// What a person approves: SHA-256 over both lists, and over the
    /// scripts when there are any (version 2), or terminals, declared ports or
    /// allocation choices (version 3,
    /// ADR 0066). The module's Decision Log
    /// says why earlier digests stay for a file without new keys.
    #[must_use]
    pub fn digest(&self) -> CommandsDigest {
        let mut hash = Sha256::new();
        let version_three = self.service_ports.is_some()
            || !self.terminals.is_empty()
            || self.scripts.iter().any(|script| script.port().is_some());
        if version_three {
            hash.update(b"vessel.toml commands 3\n");
        } else if self.scripts.is_empty() {
            hash.update(b"vessel.toml worktree commands 1\n");
        } else {
            hash.update(b"vessel.toml commands 2\n");
        }
        for (name, list) in [("setup", &self.setup), ("teardown", &self.teardown)] {
            hash.update(name.as_bytes());
            hash.update(u64::try_from(list.len()).unwrap_or(u64::MAX).to_le_bytes());
            for command in list {
                scripts::field(&mut hash, command.as_str().as_bytes());
            }
        }
        if version_three {
            scripts::digest(&mut hash, &self.scripts, scripts::DigestFields::WithPorts);
            terminals::digest(&mut hash, &self.terminals);
            // Use the reserved slots without changing terminal-only approvals.
            let range = self
                .service_ports
                .as_ref()
                .and_then(ServicePorts::range)
                .map(|range| range.to_string());
            let program = self
                .service_ports
                .as_ref()
                .and_then(ServicePorts::port_script);
            scripts::field(&mut hash, range.as_deref().unwrap_or("").as_bytes());
            scripts::field(
                &mut hash,
                program.map_or("", |program| program.as_str()).as_bytes(),
            );
        } else if !self.scripts.is_empty() {
            scripts::digest(&mut hash, &self.scripts, scripts::DigestFields::Original);
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

// Both decoders share the same file authority and byte budget; styles must
// not gain a less constrained path just because commands use another parser.
fn read_bytes(folder: &Path) -> Result<Option<Vec<u8>>, ProjectFileError> {
    let Ok(root) = Root::open(folder) else {
        return Ok(None);
    };
    let Ok(name) = FILE_NAME.parse::<RelPath>() else {
        return Ok(None);
    };
    let path = match root.existing(&name) {
        Ok(path) => path,
        Err(PathError::NotFound(_)) => return Ok(None),
        // As if there were none, as for a link that leads nowhere: an
        // answer of its own would tell whoever planted the link what
        // exists outside the folder.
        Err(PathError::Outside(_) | PathError::GitDir(_)) => {
            tracing::warn!("a project's vessel.toml leads out of its folder; not read");
            return Ok(None);
        }
        Err(PathError::Root | PathError::Platform(_) | PathError::Io(_)) => {
            return Err(ProjectFileError::NotAFile);
        }
    };
    let Some(file) = crate::regular::open(&path).map_err(ProjectFileError::Unreadable)? else {
        return Err(ProjectFileError::NotAFile);
    };
    let mut bytes = Vec::new();
    let _read = file
        .take(
            u64::try_from(MAX_BYTES)
                .unwrap_or(u64::MAX)
                .saturating_add(1),
        )
        .read_to_end(&mut bytes)
        .map_err(ProjectFileError::Unreadable)?;
    if bytes.len() > MAX_BYTES {
        return Err(ProjectFileError::TooLarge);
    }
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder's file is read only when it is a regular file inside the
    /// folder: a link out of it, or to a device, is never read; a link
    /// inside it is followed.
    #[cfg(unix)]
    #[test]
    fn a_folders_file_is_read_only_inside_it() {
        use std::path::PathBuf;

        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("token");
        std::fs::write(&secret, "[worktree]\nsetup = \"echo secret\"\n").unwrap();
        let folder = tempfile::tempdir().unwrap();
        assert!(ProjectFile::read_in(folder.path()).unwrap().is_none());

        // Out of the folder, read as none, whether the target is there or
        // not, so the answer says nothing of what exists outside.
        let file = folder.path().join(FILE_NAME);
        for target in [
            secret.clone(),
            PathBuf::from("/dev/zero"),
            outside.path().join("none"),
        ] {
            std::os::unix::fs::symlink(&target, &file).unwrap();
            assert!(
                ProjectFile::read_in(folder.path()).unwrap().is_none(),
                "{target:?}"
            );
            std::fs::remove_file(&file).unwrap();
        }
        std::fs::create_dir(&file).unwrap();
        assert!(matches!(
            ProjectFile::read_in(folder.path()),
            Err(ProjectFileError::NotAFile)
        ));
        std::fs::remove_dir(&file).unwrap();

        let inside = folder.path().join("setup.toml");
        std::fs::write(&inside, "[worktree]\nsetup = \"npm ci\"\n").unwrap();
        std::os::unix::fs::symlink(&inside, &file).unwrap();
        let parsed = ProjectFile::read_in(folder.path()).unwrap().unwrap();
        assert_eq!(parsed.setup.len(), 1);
    }

    /// A file that grows past the bound is refused, however it grew.
    #[test]
    fn a_folders_file_past_the_bound_is_refused() {
        let folder = tempfile::tempdir().unwrap();
        let mut text = "[worktree]\nsetup = \"npm ci\"\n".to_owned();
        text.push_str(&"#".repeat(MAX_BYTES));
        std::fs::write(folder.path().join(FILE_NAME), text).unwrap();
        assert!(matches!(
            ProjectFile::read_in(folder.path()),
            Err(ProjectFileError::TooLarge)
        ));
    }

    #[test]
    fn strings_lists_and_other_tables_parse() {
        let file = ProjectFile::parse(
            br#"
            # comments are fine
            [worktree]
            setup = ["npm ci", "cp \"$VESSEL_PROJECT_ROOT/.env\" ."]
            teardown = "docker compose down"

            [services.dev]
            command = "npm run dev"
            "#,
        )
        .unwrap();
        assert_eq!(file.setup.len(), 2);
        assert_eq!(file.scripts.len(), 1);
        assert_eq!(
            file.teardown.first().unwrap().as_str(),
            "docker compose down"
        );
        assert!(ProjectFile::parse(b"").unwrap().is_empty());
        assert!(
            !ProjectFile::parse(b"[scripts.x]\ncommand = 'y'")
                .unwrap()
                .is_empty(),
            "a script is a command"
        );
        assert!(ProjectFile::parse(b"[env]\nA = '1'").unwrap().is_empty());
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
    fn scripts_and_services_are_read_checked_and_ordered() {
        let file = ProjectFile::parse(
            br#"
            [scripts.test]
            command = "cargo test"
            later = "a key a newer node reads is ignored"

            [services.web]
            command = "npm run dev"
            dir = "web"

            [scripts.build]
            command = "cargo build"
            "#,
        )
        .unwrap();
        let names: Vec<&str> = file.scripts.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["build", "test", "web"]);
        let web = file.script(&"web".parse().unwrap()).unwrap();
        assert_eq!(web.kind(), vessel_proto::scripts::ScriptKind::Service);
        assert_eq!(web.dir.as_ref().map(RelPath::as_str), Some("web"));

        for bad in [
            "[scripts.a]\ndir = 'x'",
            "[scripts.a]\ncommand = 'x'\n[services.a]\ncommand = 'y'",
            "[scripts.'-a']\ncommand = 'x'",
            "[scripts.a]\ncommand = ''",
            "[scripts.a]\ncommand = 'x'\ndir = '../out'",
            "[scripts]\na = 'x'",
        ] {
            assert!(ProjectFile::parse(bad.as_bytes()).is_err(), "{bad}");
        }
        let many = (0..=MAX_SCRIPTS).fold(String::new(), |mut text, index| {
            let _written = std::fmt::Write::write_fmt(
                &mut text,
                format_args!("[scripts.s{index}]\ncommand = 'x'\n"),
            );
            text
        });
        assert!(matches!(
            ProjectFile::parse(many.as_bytes()),
            Err(ProjectFileError::Scripts(ScriptsError::TooMany))
        ));
    }

    /// An approval given before scripts existed still covers a file that
    /// names none; one with scripts never matches it.
    #[test]
    fn the_digest_grows_only_where_the_file_does() {
        let digest = |text: &str| ProjectFile::parse(text.as_bytes()).unwrap().digest();
        // The version 1 digest of this file, as protocol 10 computed it.
        let mut v1 = Sha256::new();
        v1.update(b"vessel.toml worktree commands 1\n");
        v1.update(b"setup");
        v1.update(1_u64.to_le_bytes());
        v1.update(2_u64.to_le_bytes());
        v1.update(b"ci");
        v1.update(b"teardown");
        v1.update(0_u64.to_le_bytes());
        let v1 = CommandsDigest::from_sha256(v1.finalize().into());
        assert_eq!(digest("[worktree]\nsetup = 'ci'"), v1);

        let with = digest("[worktree]\nsetup = 'ci'\n[scripts.t]\ncommand = 'x'");
        assert_ne!(with, v1);
        for changed in [
            "[worktree]\nsetup = 'ci'\n[services.t]\ncommand = 'x'",
            "[worktree]\nsetup = 'ci'\n[scripts.u]\ncommand = 'x'",
            "[worktree]\nsetup = 'ci'\n[scripts.t]\ncommand = 'y'",
            "[worktree]\nsetup = 'ci'\n[scripts.t]\ncommand = 'x'\ndir = 'd'",
            "[worktree]\nsetup = 'cj'\n[scripts.t]\ncommand = 'x'",
            "[worktree]\nsetup = 'ci'\n[scripts.t]\ncommand = 'x'\n[scripts.u]\ncommand = 'x'",
        ] {
            assert_ne!(digest(changed), with, "{changed}");
        }
        assert_eq!(
            digest("[scripts.b]\ncommand = 'y'\n[scripts.a]\ncommand = 'x'"),
            digest("[scripts.a]\ncommand = 'x'\n[scripts.b]\ncommand = 'y'"),
            "the order in the file does not matter"
        );
        assert_eq!(
            digest("[worktree]\nsetup = 'ci'\n[scripts.t]\ncommand = 'x'\nnote = 'z'"),
            with,
            "a key that does not run is not approved"
        );
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

#[cfg(test)]
mod declared_ports_tests;

#[cfg(test)]
mod allocation_tests;
