//! `settings.json` in the data directory (protocol 12, ADR 0038): the node
//! settings a person changed through `node.set_settings`.
//!
//! [Decision Log]
//! - Purpose: a setting changed from the app must survive a restart, and a
//!   service started with `vesseld --park-after` must not undo it at its
//!   next start.
//! - Choice: only what a person changed is kept (every field optional),
//!   laid over the flags' values when the node starts; a setting nobody
//!   changed keeps following its flag. The same type as the wire's
//!   change (`SetNodeSettings`), so a later setting is one more optional
//!   field here too.
//! - It is written owner-only and atomically, as `agents.toml` is, so a
//!   reader sees the old file or the new one.
//! - A file from a later build parses here: the type never refuses fields
//!   it does not know, at any level (`SetNodeSettings`'s own rule), so a
//!   downgrade keeps every setting this build knows. What it does not know
//!   (a later field, or a later value read as `unknown`) a change made here
//!   cannot write back, so the node copies the file aside first
//!   ([`DataDir::settings_hold_more_than`], `settings.json.later-<UTC
//!   time>`); nothing is merged back on its own.
//! - A file that still cannot be read or parsed is not fatal, and is not
//!   lost (2026-11-14; until then the next change replaced it, so a
//!   damaged file or a downgrade lost every saved setting at the first
//!   change): the node moves it aside first ([`DataDir::set_aside_settings`],
//!   `settings.json.unreadable-<UTC time>` in the same folder, owner-only,
//!   one rename, never over an earlier copy) and starts from its flags. If
//!   even the move fails, the node refuses to replace the file and says
//!   why on the next change. Rejected: refusing every change until a
//!   person repairs the file, which leaves a node whose file was damaged
//!   unable to change any setting from the app.

mod decode;

pub use decode::SettingsDecodeFailure;

use std::io;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use vessel_proto::settings::SetNodeSettings;

use crate::data_dir::DataDir;
use crate::error::StoreError;
use crate::files;

const SETTINGS_FILE: &str = "settings.json";

/// How many names [`DataDir::set_aside_settings`] tries for one moment
/// before it gives up: one per earlier copy set aside in the same
/// millisecond, which a node never makes more than once per start.
const SET_ASIDE_TRIES: u32 = 16;

/// Why a copy of `settings.json` is set aside ([`DataDir::set_aside_settings`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetAside {
    /// It cannot be read or parsed: moved, so no change replaces it.
    Unreadable,
    /// It holds what only a later build knows: copied before the first
    /// change this build writes replaces it.
    Later,
}

/// Why `settings.json` was not read.
#[derive(Debug, thiserror::Error)]
pub enum SettingsFileError {
    /// It could not be read.
    #[error(transparent)]
    Read(#[from] StoreError),
    /// It is not the settings' JSON.
    #[error("{}: {source}", path.display())]
    Parse {
        /// The file.
        path: PathBuf,
        /// Safe metadata; the original serde cause stays private inside it.
        #[source]
        source: SettingsDecodeFailure,
    },
}

impl DataDir {
    /// The node settings a person changed (protocol 12).
    #[must_use]
    pub fn settings_path(&self) -> PathBuf {
        self.root().join(SETTINGS_FILE)
    }

    /// The saved settings, or `None` when none were ever changed.
    ///
    /// # Errors
    /// The file exists but cannot be read, or does not parse.
    pub fn read_settings(&self) -> Result<Option<SetNodeSettings>, SettingsFileError> {
        let path = self.settings_path();
        let text = match std::fs::read(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(StoreError::io(&path)(error).into()),
        };
        decode::settings(&text)
            .map(Some)
            .map_err(|source| SettingsFileError::Parse { path, source })
    }

    /// Set `settings.json` aside as `settings.json.<why>-<at>` in the same
    /// folder, owner-only, so no change replaces what it holds; answers
    /// where it went. An unreadable file is moved (one rename: it is at one
    /// name or the other); a file from a later build is copied. An earlier copy is never
    /// replaced: a name taken (a copy set aside in the same millisecond)
    /// gets a `-2`, `-3`... suffix. The check and the rename are not one
    /// step, which is sound because only the node holding the data
    /// directory's lock calls this.
    ///
    /// # Errors
    /// There is no file to move, no free name, or the rename fails; the
    /// file then stays where it was.
    pub fn set_aside_settings(
        &self,
        at: DateTime<Utc>,
        why: SetAside,
    ) -> Result<PathBuf, StoreError> {
        let path = self.settings_path();
        let kind = match why {
            SetAside::Unreadable => "unreadable",
            SetAside::Later => "later",
        };
        let stamp = at.format("%Y%m%dT%H%M%S%.3fZ");
        for attempt in 1..=SET_ASIDE_TRIES {
            let name = if attempt == 1 {
                format!("{SETTINGS_FILE}.{kind}-{stamp}")
            } else {
                format!("{SETTINGS_FILE}.{kind}-{stamp}-{attempt}")
            };
            let aside = self.root().join(name);
            match std::fs::symlink_metadata(&aside) {
                Ok(_taken) => continue,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(StoreError::io(&aside)(error)),
            }
            match why {
                SetAside::Unreadable => std::fs::rename(&path, &aside).map(drop),
                // The file stays until the write replaces it atomically.
                SetAside::Later => std::fs::copy(&path, &aside).map(drop),
            }
            .map_err(StoreError::io(&path))?;
            // Kept as private as the file it was; a copy someone made
            // readable by others is tightened like the store's own files.
            let _privacy = files::make_private(&aside).map_err(StoreError::io(&aside))?;
            return Ok(aside);
        }
        Err(StoreError::io(&path)(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "every name to set the settings aside under is taken",
        )))
    }

    /// Whether `settings.json` holds more than `known`, what this build
    /// parsed from it: a field, at any depth, or a value only a later
    /// build knows (read here as `unknown`). No file holds nothing.
    ///
    /// # Errors
    /// The file exists but cannot be read, or is not JSON.
    pub fn settings_hold_more_than(&self, known: &SetNodeSettings) -> Result<bool, StoreError> {
        let path = self.settings_path();
        let text = match std::fs::read(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(StoreError::io(&path)(error)),
        };
        let invalid = |error| StoreError::io(&path)(io::Error::other(error));
        let file: serde_json::Value = serde_json::from_slice(&text).map_err(invalid)?;
        let known = serde_json::to_value(known).map_err(invalid)?;
        Ok(!covered(&file, &known))
    }

    /// Replace the saved settings, owner-only and atomically.
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn write_settings(&self, settings: &SetNodeSettings) -> Result<(), StoreError> {
        let path = self.settings_path();
        let text = serde_json::to_vec_pretty(settings)
            .map_err(|error| StoreError::io(&path)(io::Error::other(error)))?;
        files::replace(&path, &text).map_err(StoreError::io(&path))
    }
}

/// Whether every key of `file`, at every depth, is in `known` with the
/// same value.
fn covered(file: &serde_json::Value, known: &serde_json::Value) -> bool {
    match (file, known) {
        (serde_json::Value::Object(file), serde_json::Value::Object(known)) => file
            .iter()
            .all(|(key, value)| known.get(key).is_some_and(|known| covered(value, known))),
        (file, known) => file == known,
    }
}

#[cfg(test)]
mod tests {
    use vessel_proto::parking::{IdleMinutes, ParkAfter};
    use vessel_proto::settings::AgentDriver;

    use super::*;

    #[test]
    fn saved_settings_are_read_back_and_a_bad_file_says_why() {
        let dir = tempfile::tempdir().unwrap();
        let data = DataDir::new(dir.path());
        assert!(data.read_settings().unwrap().is_none());
        let saved = SetNodeSettings {
            browser_tools: None,
            park_after: Some(ParkAfter::Idle {
                minutes: IdleMinutes::new(15).unwrap(),
            }),
            drivers: None,
        };
        data.write_settings(&saved).unwrap();
        assert_eq!(data.read_settings().unwrap(), Some(saved));
        std::fs::write(data.settings_path(), b"{ not json").unwrap();
        assert!(matches!(
            data.read_settings(),
            Err(SettingsFileError::Parse { .. })
        ));
    }

    /// A downgrade: a later build's fields, at the top and inside
    /// `drivers`, and a driver value this build does not know.
    #[test]
    fn a_file_from_a_later_build_keeps_what_this_build_knows() {
        let dir = tempfile::tempdir().unwrap();
        let data = DataDir::new(dir.path());
        std::fs::write(
            data.settings_path(),
            br#"{
                "park_after": { "kind": "idle", "minutes": 15, "since": "later" },
                "drivers": { "codex": "direct", "claude_code": "teleport", "gemini": "acp" },
                "dictation": { "provider": "later" }
            }"#,
        )
        .unwrap();
        let read = data.read_settings().unwrap().unwrap();
        assert_eq!(
            read.park_after,
            Some(ParkAfter::Idle {
                minutes: IdleMinutes::new(15).unwrap()
            })
        );
        let drivers = read.drivers.unwrap();
        assert_eq!(drivers.codex, Some(AgentDriver::Direct));
        assert_eq!(drivers.claude_code, Some(AgentDriver::Unknown));
        assert_eq!(drivers.open_code, None);
    }

    #[test]
    fn an_unreadable_file_is_set_aside_and_never_over_an_earlier_copy() {
        let dir = tempfile::tempdir().unwrap();
        let data = DataDir::new(dir.path());
        let at = "2026-11-14T21:55:01.123Z".parse::<DateTime<Utc>>().unwrap();
        std::fs::write(data.settings_path(), b"{ first").unwrap();
        let first = data.set_aside_settings(at, SetAside::Unreadable).unwrap();
        assert_eq!(
            first.file_name().unwrap(),
            "settings.json.unreadable-20261114T215501.123Z"
        );
        assert!(!data.settings_path().exists());
        assert!(data.read_settings().unwrap().is_none());

        // A second damaged file at the same moment takes the next name.
        std::fs::write(data.settings_path(), b"{ second").unwrap();
        let second = data.set_aside_settings(at, SetAside::Unreadable).unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read(&first).unwrap(), b"{ first");
        assert_eq!(std::fs::read(&second).unwrap(), b"{ second");
        assert!(
            data.set_aside_settings(at, SetAside::Unreadable).is_err(),
            "nothing to move"
        );
    }

    #[test]
    fn a_file_holding_more_than_this_build_knows_is_told_apart() {
        let dir = tempfile::tempdir().unwrap();
        let data = DataDir::new(dir.path());
        let more = |text: &str| {
            std::fs::write(data.settings_path(), text).unwrap();
            let known = data.read_settings().unwrap().unwrap();
            data.settings_hold_more_than(&known).unwrap()
        };
        assert!(!more(
            r#"{"park_after":{"kind":"never"},"drivers":{"codex":"direct"}}"#
        ));
        assert!(more(
            r#"{"park_after":{"kind":"never"},"dictation":{"on":true}}"#
        ));
        assert!(more(r#"{"drivers":{"codex":"acp","gemini":"acp"}}"#));
        assert!(more(r#"{"drivers":{"codex":"teleport"}}"#), "a later value");
        std::fs::remove_file(data.settings_path()).unwrap();
        assert!(
            !data
                .settings_hold_more_than(&SetNodeSettings::default())
                .unwrap()
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_set_aside_copy_is_owner_only() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().unwrap();
        let data = DataDir::new(dir.path());
        std::fs::write(data.settings_path(), b"{ damaged").unwrap();
        std::fs::set_permissions(data.settings_path(), std::fs::Permissions::from_mode(0o644))
            .unwrap();
        let aside = data
            .set_aside_settings(Utc::now(), SetAside::Unreadable)
            .unwrap();
        let mode = std::fs::metadata(aside).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "{mode:o}");
    }
}
