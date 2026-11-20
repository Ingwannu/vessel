//! The file work every agent's installer shares: the one-time backup, the
//! private atomic write, the removal, the log line, and, in tests, the guard
//! that refuses a person's real agent configuration.
//!
//! [Decision Log]
//! - Purpose: these files drive a person's agents. A write must never leave
//!   one half written, must keep what was there first, and must never come
//!   from a test.
//! - Choice:
//!   - Every write goes through [`replace`], which keeps a one-time copy of
//!     the file as it was (`<file>.vessel-backup`, beside the path the
//!     agent reads) before its first change. The copy is created with
//!     `create_new`, so two writers can never overwrite it (node review
//!     R3). [`remove`] takes away only Vessel's own exact file and keeps no
//!     copy.
//!   - Writes are owner-only and atomic (`vessel_fleet::write_private`) and
//!     go to the file a link points at, as Paseo's
//!     `writePrivateFileAtomicSync` does (`server/private-files.ts:27-55`):
//!     a `settings.json` linked into a person's dotfiles stays a link, and
//!     the backup holds the target's bytes (node review R2). Renaming onto
//!     the link itself would replace it with a plain file.
//!   - In test builds, both refuse any path inside the real home's agent
//!     configuration, or under `CLAUDE_CONFIG_DIR` or `CODEX_HOME`, with a
//!     panic. A test that reached a real file would fail rather than write.

use std::io;
use std::path::{Path, PathBuf};

use super::Agent;

/// What one agent's installer did.
#[derive(Debug)]
pub(super) enum Outcome {
    /// The file already said what it should.
    Unchanged,
    /// The file was written or removed.
    Changed(PathBuf),
    /// The file is not what Vessel can read; it was left as it is.
    Invalid(PathBuf),
    /// Reading or writing failed; what was there is unchanged or backed up.
    Failed(PathBuf, io::Error),
}

/// The one-time copy kept beside `path`.
pub(super) fn backup_of(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".vessel-backup");
    PathBuf::from(name)
}

/// Keep `path` as it was before Vessel's first change: once, never
/// overwritten. Nothing to keep when `path` does not exist.
fn back_up(path: &Path) -> io::Result<()> {
    let backup = backup_of(path);
    if backup.exists() {
        return Ok(());
    }
    match std::fs::read(path) {
        Ok(contents) => create_new_private(&backup, &contents),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// A new owner-only file at `path` holding `contents`; one already there
/// is kept as it is. A failed write takes the new file away again.
fn create_new_private(path: &Path, contents: &[u8]) -> io::Result<()> {
    #[cfg(test)]
    refuse_real(path);
    let mut options = std::fs::OpenOptions::new();
    let _options = options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        let _private = options.mode(0o600);
    }
    let mut file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(()),
        Err(error) => return Err(error),
    };
    let written = io::Write::write_all(&mut file, contents).and_then(|()| file.sync_all());
    if written.is_err() {
        drop(file);
        let _removed = std::fs::remove_file(path);
    }
    written
}

/// Where a write to `path` lands: the file a link points at (Paseo's
/// `resolveSymlinkedFile`), or `path` itself when nothing is there yet.
fn destination(path: &Path) -> io::Result<PathBuf> {
    match std::fs::canonicalize(path) {
        Ok(target) => Ok(target),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(path.to_owned()),
        Err(error) => Err(error),
    }
}

fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
    #[cfg(test)]
    refuse_real(path);
    vessel_fleet::write_private(path, contents)
}

/// Write `contents` to `path` (to its target when it is a link), after its
/// one-time backup.
pub(super) fn replace(path: &Path, contents: &[u8]) -> Outcome {
    let written = back_up(path)
        .and_then(|()| destination(path))
        .and_then(|target| write(&target, contents));
    match written {
        Ok(()) => Outcome::Changed(path.to_owned()),
        Err(error) => Outcome::Failed(path.to_owned(), error),
    }
}

/// Remove Vessel's own file at `path`. No backup: the caller removes only a
/// file whose text is exactly Vessel's, and a copy of it would be a stray
/// file in the agent's folder (`OpenCode`'s `plugins`).
pub(super) fn remove(path: &Path) -> Outcome {
    #[cfg(test)]
    refuse_real(path);
    match std::fs::remove_file(path) {
        Ok(()) => Outcome::Changed(path.to_owned()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Outcome::Unchanged,
        Err(error) => Outcome::Failed(path.to_owned(), error),
    }
}

/// One line for what an agent's installer did. An invalid file is the one
/// a person must look at.
pub(super) fn log(agent: Agent, on: bool, outcome: &Outcome) {
    let action = if on { "installed" } else { "removed" };
    match outcome {
        Outcome::Unchanged => {}
        Outcome::Changed(path) => {
            tracing::info!(agent = agent.id(), path = %path.display(), "terminal agent hooks {action}");
        }
        Outcome::Invalid(path) => tracing::warn!(
            agent = agent.id(),
            path = %path.display(),
            "terminal agent hooks not {action}: the file is not what Vessel can read, so it is left as it is"
        ),
        Outcome::Failed(path, error) => tracing::warn!(
            agent = agent.id(),
            path = %path.display(),
            %error,
            "terminal agent hooks not {action}"
        ),
    }
}

/// Tests write only under a scratch home: refuse a path inside the real
/// home's agent configuration, or under the real `CLAUDE_CONFIG_DIR` or
/// `CODEX_HOME`, with a panic.
#[cfg(test)]
pub(super) fn refuse_real(path: &Path) {
    let mut real: Vec<PathBuf> = ["CLAUDE_CONFIG_DIR", "CODEX_HOME", "OPENCODE_CONFIG_DIR"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect();
    if let Some(home) = dirs::home_dir() {
        real.push(home.join(".codex"));
        real.push(home.join(".config").join("opencode"));
        let claude = path
            .strip_prefix(&home)
            .ok()
            .and_then(|rest| rest.components().next())
            .is_some_and(|first| first.as_os_str().to_string_lossy().starts_with(".claude"));
        assert!(
            !claude,
            "agent hook tests use a scratch home, never {}",
            path.display()
        );
    }
    for dir in real {
        assert!(
            !path.starts_with(&dir),
            "agent hook tests use a scratch home, never {}",
            path.display()
        );
    }
}

/// [`refuse_real`] for every agent's home.
#[cfg(test)]
pub(super) fn refuse_real_home(targets: &super::Targets) {
    for agent in Agent::ALL {
        if let Some(home) = targets.home(agent) {
            refuse_real(home);
        }
    }
}
