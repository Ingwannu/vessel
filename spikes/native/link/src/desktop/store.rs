//! One process owns window persistence; stale window preference snapshots cannot win.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vessel_app_view::desktop::window::WindowState;

use super::Error;

const VERSION: u16 = 1;
const DOCUMENT_LIMIT: u64 = 64 * 1024;

/// Explicit private storage root provided by the host binary.
#[derive(Debug, Clone)]
pub struct Store {
    directory: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u16,
    state: WindowState,
}

impl Store {
    /// Validate the host-supplied path before any directory or file is created.
    ///
    /// # Errors
    /// Relative paths are not stable across desktop launches.
    pub fn new(directory: PathBuf) -> Result<Self, Error> {
        if !directory.is_absolute() {
            return Err(Error::RelativePath);
        }
        Ok(Self { directory })
    }

    /// Share the same private root with the desktop instance owner.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// A missing file is first launch; unreadable or newer files are never replaced.
    ///
    /// # Errors
    /// Reports I/O, bounded-document, decoding and version failures.
    pub fn load_window(&self) -> Result<Option<WindowState>, Error> {
        let path = self.directory.join("window-state.json");
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if file.metadata()?.len() > DOCUMENT_LIMIT {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "window document exceeds its size limit",
            )
            .into());
        }
        let document: Document = serde_json::from_reader(file)?;
        if document.version != VERSION {
            return Err(Error::NewerDocument);
        }
        Ok(Some(document.state))
    }

    /// Write a private new file and atomically replace the old state.
    ///
    /// # Errors
    /// Refuses serialization or OS errors and removes only its own temporary file.
    pub fn save_window(&self, state: WindowState) -> Result<(), Error> {
        let _original = self.load_window()?;
        crate::prefs::private_dir(&self.directory)?;
        let path = self.directory.join("window-state.json");
        // A single process-level writer serializes these calls, including its final save.
        let temporary = self
            .directory
            .join(format!("window-state.{}.tmp", std::process::id()));
        let mut file = crate::prefs::new_private_file(&temporary)?;
        let result = (|| {
            serde_json::to_writer(
                &mut file,
                &Document {
                    version: VERSION,
                    state,
                },
            )?;
            file.flush()?;
            std::fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            drop(std::fs::remove_file(temporary));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_atomic_round_trip_preserves_an_unreadable_document() {
        let path =
            std::env::temp_dir().join(format!("vessel-desktop-store-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        let store = Store::new(path.clone()).unwrap();
        assert_eq!(store.load_window().unwrap(), None);
        store.save_window(WindowState::default()).unwrap();
        assert_eq!(store.load_window().unwrap(), Some(WindowState::default()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(path.join("window-state.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        std::fs::write(path.join("window-state.json"), "unreadable").unwrap();
        assert!(store.load_window().is_err());
        assert_eq!(
            std::fs::read_to_string(path.join("window-state.json")).unwrap(),
            "unreadable"
        );
        std::fs::remove_dir_all(path).unwrap();
    }
}
