//! Installing PATH support appends one known line and preserves the user's rc file.

use crate::desktop::DesktopError;
use std::ffi::OsString;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

const RC_LIMIT: u64 = 1024 * 1024;
const POSIX_LINE: &str = "export PATH=\"$HOME/.local/bin:$PATH\"";
const FISH_LINE: &str = "fish_add_path $HOME/.local/bin";
const MARKER: &str = "# Added by Vessel";

/// No rc edit is attempted for platforms or shells whose semantics are unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Bash login startup is `.bash_profile` on macOS.
    Mac,
    /// Bash's interactive startup is .bashrc on other supported Unix desktops.
    Unix,
    /// The Installation owner changes Windows user PATH; shell rc edits are inapplicable.
    Windows,
}

pub use vessel_app_view::desktop::cli::PathStatus;

/// PATH readiness and whether the known startup file was actually changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathMutation {
    /// A fresh terminal may still need manual configuration.
    pub status: PathStatus,
    /// Idempotent status reads never pretend they wrote an rc file.
    pub change: super::Change,
}

/// All paths and inherited PATH data come explicitly from the host binary.
#[derive(Clone)]
pub struct PathSetup {
    local_bin: PathBuf,
    rc: Option<(PathBuf, &'static str)>,
    path: OsString,
}

impl std::fmt::Debug for PathSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PathSetup").finish_non_exhaustive()
    }
}

impl PathSetup {
    /// Choose only the reference's known rc locations; never interpret shell text.
    ///
    /// # Errors
    /// A relative home cannot identify an account's startup file safely.
    pub fn new(
        home: &Path,
        shell: Option<&Path>,
        platform: Platform,
        path: OsString,
    ) -> Result<Self, DesktopError> {
        if !home.is_absolute() {
            return Err(DesktopError::RelativePath);
        }
        let rc = if platform == Platform::Windows {
            None
        } else {
            match shell
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
            {
                Some("bash") => Some((
                    home.join(if platform == Platform::Mac {
                        ".bash_profile"
                    } else {
                        ".bashrc"
                    }),
                    POSIX_LINE,
                )),
                Some("zsh") => Some((home.join(".zshrc"), POSIX_LINE)),
                Some("fish") => Some((home.join(".config/fish/config.fish"), FISH_LINE)),
                Some(_) | None => None,
            }
        };
        Ok(Self {
            local_bin: home.join(".local/bin"),
            rc,
            path,
        })
    }

    /// Read current PATH/known rc configuration without creating or editing a file.
    ///
    /// # Errors
    /// Bounded read, UTF-8 and permission failures retain their source.
    pub fn status(&self) -> Result<PathStatus, DesktopError> {
        if std::env::split_paths(&self.path)
            .any(|path| path == self.local_bin || path == Path::new("~/.local/bin"))
        {
            return Ok(PathStatus::Available);
        }
        let Some((rc, line)) = &self.rc else {
            return Ok(PathStatus::Manual);
        };
        let mut content = String::new();
        match std::fs::File::open(rc) {
            Ok(file) => {
                let _read = file
                    .take(RC_LIMIT.saturating_add(1))
                    .read_to_string(&mut content)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        if u64::try_from(content.len()).map_err(|_overflow| DesktopError::InvalidLaunch)? > RC_LIMIT
        {
            return Err(DesktopError::InvalidLaunch);
        }
        if content.lines().any(|existing| existing.trim() == *line) {
            return Ok(PathStatus::Configured);
        }
        Ok(PathStatus::Manual)
    }

    /// Append once, leaving existing ownership/mode and symlink semantics intact.
    ///
    /// # Errors
    /// Bounded reads, append and permission failures are explicit.
    pub fn ensure(&self) -> Result<PathStatus, DesktopError> {
        self.ensure_change().map(|mutation| mutation.status)
    }

    /// Preserve the existing policy while reporting actual PATH writes separately.
    ///
    /// # Errors
    /// The same bounded read and permission failures remain explicit.
    pub fn ensure_change(&self) -> Result<PathMutation, DesktopError> {
        let status = self.status()?;
        if status != PathStatus::Manual {
            return Ok(PathMutation {
                status,
                change: super::Change::Unchanged,
            });
        }
        let Some((rc, line)) = &self.rc else {
            return Ok(PathMutation {
                status: PathStatus::Manual,
                change: super::Change::Unchanged,
            });
        };
        if let Some(parent) = rc.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = match std::fs::OpenOptions::new().append(true).open(rc) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                crate::prefs::new_private_file(rc)?
            }
            Err(error) => return Err(error.into()),
        };
        file.write_all(format!("\n{MARKER}\n{line}\n").as_bytes())?;
        Ok(PathMutation {
            status: PathStatus::Configured,
            change: super::Change::Changed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_setup_is_idempotent_and_preserves_existing_rc_content() {
        let home = std::env::temp_dir().join(format!("vessel-cli-path-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(".bashrc"), "# My settings\n").unwrap();
        let plan = PathSetup::new(
            &home,
            Some(Path::new("/bin/bash")),
            Platform::Unix,
            OsString::new(),
        )
        .unwrap();
        assert_eq!(plan.status().unwrap(), PathStatus::Manual);
        assert_eq!(
            std::fs::read_to_string(home.join(".bashrc")).unwrap(),
            "# My settings\n"
        );
        let changed = plan.ensure_change().unwrap();
        assert_eq!(changed.status, PathStatus::Configured);
        assert_eq!(changed.change, super::super::Change::Changed);
        assert_eq!(plan.status().unwrap(), PathStatus::Configured);
        let first = std::fs::read(home.join(".bashrc")).unwrap();
        assert_eq!(
            plan.ensure_change().unwrap().change,
            super::super::Change::Unchanged
        );
        assert_eq!(plan.ensure().unwrap(), PathStatus::Configured);
        assert_eq!(std::fs::read(home.join(".bashrc")).unwrap(), first);
        assert!(
            String::from_utf8(first)
                .unwrap()
                .starts_with("# My settings\n")
        );
        let windows = PathSetup::new(&home, None, Platform::Windows, OsString::new()).unwrap();
        assert_eq!(windows.ensure().unwrap(), PathStatus::Manual);
        std::fs::remove_dir_all(home).unwrap();
    }
}
