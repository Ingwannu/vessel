//! Service paths and their unit-format boundary belong to the installer.
//! Never turn lossy OS text into an executable or a different data directory.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use vessel_remote::RemoteCommand;

/// Report a field name without exposing the person's path or environment.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Field {
    /// Trust the invoking executable when writing a service.
    Executable,
    /// The manager must receive this selected directory.
    Directory,
    /// Displayed removal guidance must stay one shell line.
    Unit,
    /// Service PATH must retain the installer's bytes faithfully.
    Search,
}

impl fmt::Display for Field {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Self::Executable => "executable",
            Self::Directory => "data directory",
            Self::Unit => "service file path",
            Self::Search => "PATH",
        })
    }
}

/// Unit formats must refuse paths they cannot represent before writing.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ServiceError {
    /// Preserve the original filesystem or current-directory cause.
    #[error("cannot resolve user service paths")]
    Io(#[from] io::Error),
    /// OS strings have no Unicode conversion error carrying a causal source.
    #[error("service {0} must be Unicode without control characters; no service file was written")]
    Text(Field),
}

/// Parsing and generation use one platform choice and one location rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Manager {
    /// Only the invoking user's service namespace is relevant.
    Systemd,
    /// Agents are per user; custom jobs remain the person's responsibility.
    Launchd,
}

impl Manager {
    /// Other systems retain the installer's service-unavailable outcome.
    pub(crate) const fn current() -> Option<Self> {
        if cfg!(target_os = "linux") {
            Some(Self::Systemd)
        } else if cfg!(target_os = "macos") {
            Some(Self::Launchd)
        } else {
            None
        }
    }

    /// Installation and observation must not disagree on which file they mean.
    ///
    /// # Errors
    /// The current directory cannot be resolved for a relative user home.
    pub(crate) fn file(self, home: &Path, config: Option<&Path>) -> Result<PathBuf, ServiceError> {
        let file = match self {
            Self::Systemd => config
                .filter(|path| path.is_absolute())
                .map_or_else(|| home.join(".config"), Path::to_owned)
                .join("systemd/user")
                .join(super::UNIT),
            Self::Launchd => home
                .join("Library/LaunchAgents")
                .join(format!("{}.plist", super::LABEL)),
        };
        Ok(std::path::absolute(file)?)
    }

    /// Reuse the existing POSIX value quoting for a displayed undo instruction.
    /// This returns text only; neither caller executes it.
    ///
    /// # Errors
    /// The file path cannot be represented safely in a unit comment or terminal.
    pub(super) fn undo(self, file: &Path) -> Result<RemoteCommand, ServiceError> {
        let text = path_text(file, Field::Unit)?;
        Ok(match self {
            Self::Systemd => RemoteCommand::program("systemctl")
                .arg("--user")
                .arg("disable")
                .arg("--now")
                .arg(super::UNIT)
                .arg("&&")
                .arg("rm")
                .value(text),
            Self::Launchd => RemoteCommand::program("launchctl")
                .arg("unload")
                .arg("-w")
                .value(text)
                .arg("&&")
                .arg("rm")
                .value(text),
        })
    }
}

/// Only the binary reads environment; values are validated before rendering.
///
/// # Errors
/// A path or PATH is not representable in a unit.
pub(super) fn validate_paths(exe: &Path, directory: &Path) -> Result<(), ServiceError> {
    let _exe = path_text(exe, Field::Executable)?;
    let _directory = path_text(directory, Field::Directory)?;
    Ok(())
}

/// The renderer consumes opaque environment text, not interpreted path components.
///
/// # Errors
/// PATH cannot be represented without changing its bytes.
pub(super) fn validate_search(text: &str) -> Result<(), ServiceError> {
    validate_text(text, Field::Search)
}

fn path_text(path: &Path, field: Field) -> Result<&str, ServiceError> {
    let text = path.to_str().ok_or(ServiceError::Text(field))?;
    validate_text(text, field)?;
    Ok(text)
}

fn validate_text(text: &str, field: Field) -> Result<(), ServiceError> {
    if text.chars().any(char::is_control) {
        Err(ServiceError::Text(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_paths_and_removal_guidance_keep_literal_values() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let systemd = Manager::Systemd.file(home, None).unwrap();
        assert_eq!(systemd, home.join(".config/systemd/user/vessel.service"));
        assert_eq!(
            Manager::Systemd
                .file(home, Some(Path::new("relative")))
                .unwrap(),
            systemd
        );
        let custom = home.join("it's a config");
        assert_eq!(
            Manager::Systemd.file(home, Some(&custom)).unwrap(),
            custom.join("systemd/user/vessel.service")
        );
        let launchd = Manager::Launchd.file(home, None).unwrap();
        assert_eq!(launchd, home.join("Library/LaunchAgents/vessel.node.plist"));
        let undo = Manager::Launchd.undo(&custom).unwrap();
        assert!(undo.as_str().contains("'\\''"));
    }

    #[test]
    fn service_paths_refuse_control_characters_and_non_unicode_without_echoing_them() {
        assert!(matches!(
            validate_paths(Path::new("/example/vessel"), Path::new("/example/a\nb")),
            Err(ServiceError::Text(Field::Directory))
        ));
        assert!(matches!(
            Manager::Systemd.undo(Path::new("/example/a\nb")),
            Err(ServiceError::Text(Field::Unit))
        ));
        assert!(matches!(
            validate_text("/example/a\nb", Field::Search),
            Err(ServiceError::Text(Field::Search))
        ));
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt as _;
            let invalid = PathBuf::from(std::ffi::OsString::from_vec(
                b"/example/invalid-\xff".to_vec(),
            ));
            let error = validate_paths(Path::new("/example/vessel"), &invalid).unwrap_err();
            assert!(matches!(error, ServiceError::Text(Field::Directory)));
            assert!(!error.to_string().contains("invalid-"));
        }
    }
}
