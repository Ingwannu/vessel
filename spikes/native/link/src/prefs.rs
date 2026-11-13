//! What the app remembers between launches: the machine, agent and folder
//! a new conversation last started with, and the choices each agent last
//! started with on each machine (the references remember the same: GOAT
//! its draft's provider and project, agent-orchestrator its last agent,
//! the page the last start's agent and choices per machine,
//! `web/src/data/defaults.ts`), in a small file of the app's own beside
//! the node book.
//!
//! The page keeps its choices with the bridge, by node id; the app has no
//! bridge, so it keeps them here by the machine's name in the book (a
//! renamed machine starts over). Bounded as the page bounds them.
//!
//! JSON rather than TOML: nobody edits it by hand, and it is replaced
//! whole on every save (written to a temporary file, then renamed, so a
//! crash never leaves half a file).

use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use vessel_proto::{ChoiceValue, HarnessRef, OptionChoice};

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
    /// The choices of the last start of each agent, by machine.
    #[serde(default)]
    pub choices: BTreeMap<String, Vec<Kept>>,
    /// Updates only when the person checks ("Update automatically" off;
    /// `false`, the default, is on, as Zed's `auto_update` is).
    #[serde(default)]
    pub manual_updates: bool,
    /// The build that ran last: a different one now was updated to, and
    /// says so once.
    #[serde(default)]
    pub last_label: Option<String>,
    /// An installed update's notes, kept until the build they describe
    /// runs (the updater knows them only in the build that downloaded it).
    #[serde(default)]
    pub update_notes: Option<UpdateNotes>,
}

/// A build's notes, as its feed carried them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateNotes {
    /// The build.
    pub label: String,
    /// Its notes.
    pub notes: String,
}

/// One agent's choices on one machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kept {
    /// The agent.
    pub agent: HarnessRef,
    /// What it started with that differs from its own defaults.
    pub choices: Vec<OptionChoice>,
}

/// The most choices kept for an agent (the page's `MAX_CHOICES`).
const MAX_CHOICES: usize = 8;

/// The longest id or value kept, in UTF-8 bytes (`MAX_CHOICE_BYTES`).
const MAX_CHOICE_BYTES: usize = 64;

fn fits(choice: &OptionChoice) -> bool {
    choice.id.as_str().len() <= MAX_CHOICE_BYTES
        && match &choice.value {
            ChoiceValue::Select { value } => value.as_str().len() <= MAX_CHOICE_BYTES,
            ChoiceValue::Toggle { .. } => true,
        }
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

impl Prefs {
    /// The choices `agent` last started with on `machine`, when kept.
    #[must_use]
    pub fn choices(&self, machine: &str, agent: &HarnessRef) -> Option<&[OptionChoice]> {
        self.choices
            .get(machine)?
            .iter()
            .find(|kept| &kept.agent == agent)
            .map(|kept| kept.choices.as_slice())
    }

    /// Keep the choices a start of `agent` on `machine` was made with
    /// (those past the bounds are not kept).
    pub fn keep_choices(&mut self, machine: &str, agent: &HarnessRef, choices: &[OptionChoice]) {
        let kept: Vec<OptionChoice> = choices
            .iter()
            .filter(|choice| fits(choice))
            .take(MAX_CHOICES)
            .cloned()
            .collect();
        let agents = self.choices.entry(machine.to_owned()).or_default();
        agents.retain(|earlier| &earlier.agent != agent);
        agents.push(Kept {
            agent: agent.clone(),
            choices: kept,
        });
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
