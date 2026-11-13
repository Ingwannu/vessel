//! What the updater remembers between runs: whether automatic updates are
//! on, and which build ran last.
//!
//! Zed keeps both: an `auto_update` setting (on by default; off leaves only
//! the manual check) and the version that ran last, from which it tells
//! the person once, after an update, what it was updated to. Vessel's app
//! has no preferences store yet, so the updater keeps its two values in a
//! small JSON file of its own beside the node book
//! (`<config>/vessel/app-updates.json`, the app picks the directory).
//!
//! A missing or unreadable file is the default (automatic, nothing ran
//! before): the file is a convenience, and a broken one must never stop
//! updates or the app. It is written whole to a temporary name and renamed,
//! so a crash leaves the old file or the new one.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::label::Label;

/// The largest file read; the real one is a few dozen bytes.
const MAX_BYTES: u64 = 4096;

/// The updater's memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memory {
    /// Whether the app checks by itself (at start and hourly). Off leaves
    /// only the check the person asks for.
    #[serde(default = "on")]
    pub automatic: bool,
    /// The build that ran last.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run: Option<Label>,
}

const fn on() -> bool {
    true
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            automatic: true,
            last_run: None,
        }
    }
}

impl Memory {
    /// Read the memory at `path`, or the default when it is missing,
    /// unreadable or not this format.
    ///
    /// Blocking file I/O.
    #[must_use]
    pub fn load(path: &Path) -> Self {
        let read = std::fs::metadata(path)
            .ok()
            .filter(|meta| meta.len() <= MAX_BYTES)
            .and_then(|_| std::fs::read(path).ok());
        match read {
            Some(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                tracing::debug!(%error, "the update memory does not parse; using the default");
                Self::default()
            }),
            None => Self::default(),
        }
    }

    /// Write the memory to `path` (temporary file, then rename).
    ///
    /// Blocking file I/O.
    ///
    /// # Errors
    /// The I/O error when the directory cannot be made or written.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        let mut temporary = path.as_os_str().to_owned();
        temporary.push(format!(".{}.tmp", std::process::id()));
        std::fs::write(&temporary, text)?;
        std::fs::rename(&temporary, path)
    }

    /// Note that `running` runs now. The label it was updated to, when the
    /// build that ran last was older: the app says so once.
    pub fn note_run(&mut self, running: &Label) -> Option<Label> {
        let updated = self
            .last_run
            .as_ref()
            .is_some_and(|last| last < running)
            .then(|| running.clone());
        self.last_run = Some(running.clone());
        updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(text: &str) -> Label {
        text.parse().unwrap()
    }

    #[test]
    fn a_missing_or_broken_file_is_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app-updates.json");
        assert_eq!(Memory::load(&path), Memory::default());
        std::fs::write(&path, b"{ not json").unwrap();
        assert_eq!(Memory::load(&path), Memory::default());
        std::fs::write(&path, vec![b' '; 5000]).unwrap();
        assert_eq!(Memory::load(&path), Memory::default());
    }

    #[test]
    fn it_round_trips_and_keeps_the_preference() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vessel").join("app-updates.json");
        let memory = Memory {
            automatic: false,
            last_run: Some(label("0.0.0.0.3")),
        };
        memory.save(&path).unwrap();
        assert_eq!(Memory::load(&path), memory);
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1
        );
    }

    #[test]
    fn an_update_is_announced_once() {
        let mut memory = Memory::default();
        // The first run after installing says nothing.
        assert_eq!(memory.note_run(&label("0.0.0.0.3")), None);
        assert_eq!(memory.note_run(&label("0.0.0.0.3")), None);
        assert_eq!(
            memory.note_run(&label("0.0.0.0.4")),
            Some(label("0.0.0.0.4"))
        );
        assert_eq!(memory.note_run(&label("0.0.0.0.4")), None);
        // An older build run by hand is not an update.
        assert_eq!(memory.note_run(&label("0.0.0.0.2")), None);
    }
}
