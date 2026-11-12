//! `agents.toml` in the data directory (protocol 11, ADR 0032): the agents
//! the node's owner added by their command.
//!
//! [Decision Log]
//! - Purpose: the file holds commands the node runs as its own user, so who
//!   can change it matters more than who can read it.
//! - Constraint: `profiles.toml` and `models.toml` are read with a warning
//!   when others can read them (their danger is a secret seen); here the
//!   danger is a command changed by someone else.
//! - Choice: on Unix the node refuses the file when another user owns it,
//!   or when others can write it or the data directory; one others can only
//!   read is used with a warning, as `profiles.toml` is. On Windows the
//!   data directory's inherited ACL is relied on, as for every file there.
//!   The node writes it 0600 and atomically, so a reader sees the old file
//!   or the new one.

use std::io;
use std::path::{Path, PathBuf};

use crate::data_dir::{DataDir, ProfilesFile};
use crate::error::StoreError;
use crate::files;

const AGENTS_FILE: &str = "agents.toml";

/// Why `agents.toml` was not read.
#[derive(Debug, thiserror::Error)]
pub enum AgentsFileError {
    /// It could not be read, or is not UTF-8.
    #[error(transparent)]
    Read(#[from] StoreError),
    /// Someone other than the node's user could change it: the file or the
    /// data directory is writable by others, or another user owns it.
    #[error("{} could be changed by another user ({reason}); the node runs its commands, so it refuses it", path.display())]
    Unsafe {
        /// The file.
        path: PathBuf,
        /// What makes it so.
        reason: &'static str,
    },
}

impl DataDir {
    /// The agents added by their command (ADR 0032).
    #[must_use]
    pub fn agents_path(&self) -> PathBuf {
        self.root().join(AGENTS_FILE)
    }

    /// `agents.toml`, or `None` if there is none (the node then runs only
    /// the four agents it knows).
    ///
    /// # Errors
    /// The file exists but cannot be read, is not UTF-8, or could be
    /// changed by another user ([`AgentsFileError::Unsafe`]).
    pub fn read_agents(&self) -> Result<Option<ProfilesFile>, AgentsFileError> {
        let path = self.agents_path();
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(StoreError::io(&path)(error).into()),
        };
        if let Some(reason) = unsafe_to_run(self.root(), &path).map_err(StoreError::io(&path))? {
            return Err(AgentsFileError::Unsafe { path, reason });
        }
        let readable_by_others = files::readable_by_others(&path).unwrap_or(false);
        if readable_by_others {
            tracing::warn!(
                path = %path.display(),
                "agents.toml is readable by other users and may hold secrets; run `chmod 600` on it"
            );
        }
        Ok(Some(ProfilesFile {
            text,
            readable_by_others,
        }))
    }

    /// Write `agents.toml`, owner-only and atomically.
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn write_agents(&self, text: &str) -> Result<(), StoreError> {
        let path = self.agents_path();
        files::replace(&path, text.as_bytes()).map_err(StoreError::io(&path))
    }
}

/// Why another user could change `file` (in the data directory `dir`), if
/// one could.
#[cfg(unix)]
fn unsafe_to_run(dir: &Path, file: &Path) -> io::Result<Option<&'static str>> {
    use std::os::unix::fs::MetadataExt as _;

    let dir = std::fs::metadata(dir)?;
    let file = std::fs::metadata(file)?;
    Ok(if file.uid() != dir.uid() {
        Some("another user owns it")
    } else if file.mode() & 0o022 != 0 {
        Some("others can write it")
    } else if dir.mode() & 0o022 != 0 {
        Some("others can write the data directory")
    } else {
        None
    })
}

#[cfg(not(unix))]
fn unsafe_to_run(_dir: &Path, _file: &Path) -> io::Result<Option<&'static str>> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_read_written_and_refused_when_others_can_change_it() {
        let root = std::env::temp_dir().join(format!("vessel-agents-file-{}", std::process::id()));
        files::create_private_dir(&root).unwrap();
        let dir = DataDir::new(root.clone());
        assert!(dir.read_agents().unwrap().is_none());
        dir.write_agents("[agents.a]\nprogram = \"x\"\n").unwrap();
        let read = dir.read_agents().unwrap().unwrap();
        assert!(read.text.contains("[agents.a]"));
        assert!(!read.readable_by_others);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let path = dir.agents_path();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o622)).unwrap();
            assert!(matches!(
                dir.read_agents(),
                Err(AgentsFileError::Unsafe { .. })
            ));
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(dir.read_agents().unwrap().unwrap().readable_by_others);
        }
        drop(std::fs::remove_dir_all(&root));
    }
}
