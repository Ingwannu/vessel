//! CLI installation owns its exact Unix symlink or Windows user PATH entry.

#[cfg(any(windows, test))]
mod windows_path;

pub mod path_setup;

use std::path::{Path, PathBuf};

use super::DesktopError;

pub use vessel_app_view::desktop::cli::Status;

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
    /// Refuses relative paths and directories that cannot be one Windows PATH entry.
    pub fn new(source: PathBuf, target: PathBuf) -> Result<Self, DesktopError> {
        if !source.is_absolute() || !target.is_absolute() {
            return Err(DesktopError::RelativePath);
        }
        #[cfg(windows)]
        windows_path::validate(&target)?;
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
    #[cfg(unix)]
    pub fn status(&self) -> Result<Status, DesktopError> {
        match std::fs::symlink_metadata(&self.target) {
            Ok(metadata) => self.classify(&metadata),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Status::Absent),
            Err(error) => Err(error.into()),
        }
    }

    #[cfg(unix)]
    fn classify(&self, metadata: &std::fs::Metadata) -> Result<Status, DesktopError> {
        Ok(
            if metadata.file_type().is_symlink() && std::fs::read_link(&self.target)? == self.source
            {
                Status::Installed
            } else {
                Status::Foreign
            },
        )
    }

    /// Inspect the user PATH without reading or changing the process environment.
    ///
    /// # Errors
    /// Invalid registry types, malformed data and OS failures remain explicit.
    #[cfg(windows)]
    pub fn status(&self) -> Result<Status, DesktopError> {
        windows_path::status(&self.target)
    }

    /// Install only a real bundled CLI; never overwrite an unrelated existing entry.
    ///
    /// # Errors
    /// Refuses missing source, foreign ownership and OS errors.
    pub fn install(&self) -> Result<Status, DesktopError> {
        if !self.source.is_file() {
            return Err(DesktopError::MissingCli);
        }
        match self.status()? {
            Status::Installed => return Ok(Status::Installed),
            Status::Foreign => return Err(DesktopError::ForeignCli),
            Status::Absent => {}
        }
        #[cfg(unix)]
        if let Some(parent) = self.target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.create()?;
        self.status()
    }

    #[cfg(unix)]
    fn create(&self) -> Result<(), DesktopError> {
        std::os::unix::fs::symlink(&self.source, &self.target)?;
        Ok(())
    }

    #[cfg(windows)]
    fn create(&self) -> Result<(), DesktopError> {
        windows_path::apply(
            &self.target,
            vessel_app_view::desktop::cli::Operation::Install,
        )
    }

    /// Preserve every file that no longer exactly matches this installation.
    ///
    /// # Errors
    /// Foreign ownership and OS failures are reported explicitly.
    pub fn uninstall(&self) -> Result<Status, DesktopError> {
        match self.status()? {
            Status::Absent => Ok(Status::Absent),
            Status::Foreign => Err(DesktopError::ForeignCli),
            Status::Installed => {
                #[cfg(unix)]
                std::fs::remove_file(&self.target)?;
                #[cfg(windows)]
                windows_path::apply(
                    &self.target,
                    vessel_app_view::desktop::cli::Operation::Remove,
                )?;
                Ok(Status::Absent)
            }
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn install_and_remove_never_overwrite_a_foreign_cli() {
        let root = std::env::temp_dir().join(format!("vessel-desktop-cli-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("bundled-cli");
        let target = root.join("vessel");
        let install = Installation::new(source.clone(), target.clone()).unwrap();
        assert!(matches!(install.install(), Err(DesktopError::MissingCli)));
        std::fs::write(&source, "bundled").unwrap();
        assert_eq!(install.install().unwrap(), Status::Installed);
        assert_eq!(install.uninstall().unwrap(), Status::Absent);
        std::fs::write(&target, "foreign").unwrap();
        assert!(matches!(install.install(), Err(DesktopError::ForeignCli)));
        assert!(matches!(install.uninstall(), Err(DesktopError::ForeignCli)));
        assert_eq!(std::fs::read(&target).unwrap(), b"foreign");
        std::fs::remove_dir_all(root).unwrap();
    }
}
