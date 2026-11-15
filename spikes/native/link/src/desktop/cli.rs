//! CLI installation owns only its exact symlink or Windows command trampoline.

pub mod path_setup;

use std::path::{Path, PathBuf};

use super::Error;

pub use vessel_app_view::desktop::cli::Status;

/// The GUI is Vessel.exe on Windows, so a sibling vessel.exe would recurse.
///
/// # Errors
/// The host executable must have an absolute location.
pub fn bundled_cli_path(
    executable: &Path,
    platform: path_setup::Platform,
) -> Result<PathBuf, Error> {
    if !executable.is_absolute() {
        return Err(Error::RelativePath);
    }
    Ok(executable.with_file_name(match platform {
        path_setup::Platform::Windows => "vessel-cli.exe",
        path_setup::Platform::Mac | path_setup::Platform::Unix => "vessel",
    }))
}

/// Both paths are supplied explicitly so onboarding cannot accidentally use root's home.
#[derive(Debug, Clone)]
pub struct Installation {
    source: PathBuf,
    target: PathBuf,
}

impl Installation {
    /// Paths are validated before any install side effect.
    ///
    /// # Errors
    /// Refuses relative paths and Windows command metacharacters in the source.
    pub fn new(source: PathBuf, target: PathBuf) -> Result<Self, Error> {
        if !source.is_absolute() || !target.is_absolute() {
            return Err(Error::RelativePath);
        }
        #[cfg(windows)]
        if source
            .to_str()
            .is_none_or(|s| s.contains(['"', '%', '\r', '\n']))
        {
            return Err(Error::RelativePath);
        }
        Ok(Self { source, target })
    }

    /// A release can report the location without exposing or starting a process.
    #[must_use]
    pub fn target(&self) -> &Path {
        &self.target
    }

    /// A missing bundled source disables installation while retaining safe removal.
    #[must_use]
    pub fn has_bundled_cli(&self) -> bool {
        self.source.is_file()
    }

    /// Inspect ownership without following an installed symlink.
    ///
    /// # Errors
    /// OS errors are distinct from absence.
    pub fn status(&self) -> Result<Status, Error> {
        match std::fs::symlink_metadata(&self.target) {
            Ok(metadata) => self.classify(&metadata),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Status::Absent),
            Err(error) => Err(error.into()),
        }
    }

    #[cfg(unix)]
    fn classify(&self, metadata: &std::fs::Metadata) -> Result<Status, Error> {
        Ok(
            if metadata.file_type().is_symlink() && std::fs::read_link(&self.target)? == self.source
            {
                Status::Installed
            } else {
                Status::Foreign
            },
        )
    }

    #[cfg(windows)]
    fn classify(&self, metadata: &std::fs::Metadata) -> Result<Status, Error> {
        const MAX_SHIM_BYTES: u64 = 8192;
        Ok(
            if metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.len() <= MAX_SHIM_BYTES
                && std::fs::read(&self.target)? == self.trampoline().as_bytes()
            {
                Status::Installed
            } else {
                Status::Foreign
            },
        )
    }

    /// Install only a real bundled CLI; never overwrite an unrelated existing entry.
    ///
    /// # Errors
    /// Refuses missing source, foreign ownership and OS errors.
    pub fn install(&self) -> Result<Status, Error> {
        if !self.source.is_file() {
            return Err(Error::MissingCli);
        }
        match self.status()? {
            Status::Installed => return Ok(Status::Installed),
            Status::Foreign => return Err(Error::ForeignCli),
            Status::Absent => {}
        }
        if let Some(parent) = self.target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.create()?;
        self.status()
    }

    #[cfg(unix)]
    fn create(&self) -> Result<(), Error> {
        std::os::unix::fs::symlink(&self.source, &self.target)?;
        Ok(())
    }

    #[cfg(windows)]
    fn create(&self) -> Result<(), Error> {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.target)?;
        file.write_all(self.trampoline().as_bytes())?;
        Ok(())
    }

    #[cfg(windows)]
    fn trampoline(&self) -> String {
        format!(
            "@echo off\r\nsetlocal DisableDelayedExpansion\r\n\"{}\" %*\r\nexit /b %errorlevel%\r\n",
            self.source.display()
        )
    }

    /// Preserve every file that no longer exactly matches this installation.
    ///
    /// # Errors
    /// Foreign ownership and OS failures are reported explicitly.
    pub fn uninstall(&self) -> Result<Status, Error> {
        match self.status()? {
            Status::Absent => Ok(Status::Absent),
            Status::Foreign => Err(Error::ForeignCli),
            Status::Installed => {
                std::fs::remove_file(&self.target)?;
                Ok(Status::Absent)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_and_remove_never_overwrite_a_foreign_cli() {
        let root = std::env::temp_dir().join(format!("vessel-desktop-cli-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("bundled-cli");
        let target = root.join("vessel");
        let install = Installation::new(source.clone(), target.clone()).unwrap();
        assert!(matches!(install.install(), Err(Error::MissingCli)));
        std::fs::write(&source, "bundled").unwrap();
        assert_eq!(install.install().unwrap(), Status::Installed);
        assert_eq!(install.uninstall().unwrap(), Status::Absent);
        std::fs::write(&target, "foreign").unwrap();
        assert!(matches!(install.install(), Err(Error::ForeignCli)));
        assert!(matches!(install.uninstall(), Err(Error::ForeignCli)));
        assert_eq!(std::fs::read(&target).unwrap(), b"foreign");
        std::fs::remove_dir_all(root).unwrap();
    }
}
