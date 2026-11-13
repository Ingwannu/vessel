//! What the app remembers between launches: the machine, agent and folder
//! a new conversation last started with (the references remember the
//! same: GOAT its draft's provider and project, agent-orchestrator its
//! last agent), in a small file of the app's own beside the node book.
//!
//! JSON rather than TOML: nobody edits it by hand, and it is replaced
//! whole on every save (written to a temporary file, then renamed, so a
//! crash never leaves half a file).

use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use vessel_proto::HarnessRef;

use crate::LinkError;
#[cfg(not(target_arch = "wasm32"))]
use crate::nodes::config_dir;

/// The remembered choices.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prefs {
    /// The machine of the last conversation started.
    #[serde(default)]
    pub machine: Option<String>,
    /// Its agent.
    #[serde(default)]
    pub agent: Option<HarnessRef>,
    /// The last folder used on each machine (none: its home folder).
    #[serde(default)]
    pub folders: BTreeMap<String, String>,
}

// A browser has no files: the choices last for the page's life only (the
// page keeps its own in `localStorage`; the browser build would do the
// same through `web-sys`, docs/plans/backend-m12-browser.md).
#[cfg(target_arch = "wasm32")]
impl Prefs {
    /// Nothing remembered in a browser yet.
    #[must_use]
    pub fn load() -> Self {
        Self::default()
    }

    /// Nothing written in a browser yet.
    ///
    /// # Errors
    /// None.
    pub const fn save(&self) -> Result<(), LinkError> {
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Prefs {
    fn path() -> Result<PathBuf, LinkError> {
        Ok(config_dir()?.join("vessel").join("app-prefs.json"))
    }

    /// The remembered choices; nothing remembered when the file is
    /// missing or unreadable (a broken file is replaced on the next save).
    #[must_use]
    pub fn load() -> Self {
        let Ok(path) = Self::path() else {
            return Self::default();
        };
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                tracing::warn!(%error, "the app's remembered choices could not be read");
                Self::default()
            }),
            Err(_missing) => Self::default(),
        }
    }

    /// Write them (blocking: called on the link's blocking pool).
    ///
    /// # Errors
    /// No configuration directory, or the file could not be written.
    pub fn save(&self) -> Result<(), LinkError> {
        let path = Self::path()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|error| LinkError::Io(error.to_string()))?;
        }
        let text =
            serde_json::to_vec_pretty(self).map_err(|error| LinkError::Io(error.to_string()))?;
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, text).map_err(|error| LinkError::Io(error.to_string()))?;
        std::fs::rename(&temporary, &path).map_err(|error| LinkError::Io(error.to_string()))
    }
}
