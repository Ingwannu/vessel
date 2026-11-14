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
//!
//! The person's settings (Settings > General and Appearance, and the
//! sections of the other tracks: `vessel_app_view::prefs::Preferences`,
//! the bridge's document field for field) are kept in the same file, as
//! its `settings`: read as `prefs.settings`, changed only through
//! [`Prefs::prefer`] (the bridge's rule for a change), then saved. That
//! one path is what moves when a node keeps them (ADR 0036,
//! docs/plans/backend-m12-settings.md).
//!
//! The file is the person's own, as the credential store beside it: on
//! Unix it is written 0600 whatever the caller's umask, and the folder
//! is made 0700 when the app makes it (a folder that exists keeps its
//! mode: it may be shared with other things the person keeps there).
//! Nothing secret is in it, but the app's folder is private. Windows
//! keeps the profile's inherited ACL.
//!
//! A file that cannot be read is not written over: the settings are then
//! kept until the app quits, and Settings says so and where the file is,
//! so the person can fix or remove it (the page's rule, ADR 0028).

use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vessel_app_view::prefs::{self, PreferenceChange, Preferences, Refused};
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
    /// The person's settings.
    #[serde(default)]
    pub settings: Preferences,
    /// Where all of this is kept (not in the file: found when it is read).
    #[serde(skip)]
    pub kept: prefs::Kept,
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
            ChoiceValue::Select { value, .. } => value.as_str().len() <= MAX_CHOICE_BYTES,
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
    /// Change one setting as the bridge would (refused past a bound, the
    /// others kept); save after.
    ///
    /// # Errors
    /// The change would grow the shortcuts or the machines' defaults past
    /// their bounds.
    pub fn prefer(&mut self, change: &PreferenceChange) -> Result<(), Refused> {
        self.settings = self.settings.changed(change)?;
        Ok(())
    }

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

    /// The remembered choices, and where they are kept; nothing remembered
    /// when the file is missing, or when it cannot be read (then kept
    /// until the app quits, the file left as it is).
    #[must_use]
    pub fn load() -> Self {
        Self::path().map_or_else(|_none| Self::default(), |path| Self::load_at(&path))
    }

    fn load_at(path: &Path) -> Self {
        let shown = path.display().to_string();
        let unreadable = |error: &serde_json::Error| prefs::Kept::ThisLaunch {
            why: prefs::LaunchOnly::Unreadable {
                fault: vessel_app_view::files::FileFault {
                    path: shown.clone(),
                    at: Some((error.line(), error.column())),
                    problem: vessel_app_view::files::FileProblem::of_json(
                        &error.to_string(),
                        matches!(
                            error.classify(),
                            serde_json::error::Category::Syntax | serde_json::error::Category::Eof
                        ),
                    ),
                    field: None,
                },
            },
        };
        let (read, kept) = match std::fs::read(path) {
            Ok(bytes) => match serde_json::from_slice::<Self>(&bytes) {
                Ok(read) => (
                    read,
                    prefs::Kept::File {
                        path: shown.clone(),
                    },
                ),
                Err(error) => {
                    // Where only: the reader's message may quote the file.
                    tracing::warn!(
                        line = error.line(),
                        column = error.column(),
                        "the app's remembered choices could not be read"
                    );
                    (Self::default(), unreadable(&error))
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (
                Self::default(),
                prefs::Kept::File {
                    path: shown.clone(),
                },
            ),
            Err(error) => {
                tracing::warn!(%error, "the app's remembered choices could not be read");
                let fault = vessel_app_view::files::FileFault {
                    path: shown.clone(),
                    at: None,
                    problem: vessel_app_view::files::FileProblem::CannotOpen,
                    field: None,
                };
                (
                    Self::default(),
                    prefs::Kept::ThisLaunch {
                        why: prefs::LaunchOnly::Unreadable { fault },
                    },
                )
            }
        };
        Self { kept, ..read }
    }

    /// Write them (blocking: called on the link's blocking pool).
    ///
    /// # Errors
    /// No configuration directory, or the file could not be written.
    pub fn save(&self) -> Result<(), LinkError> {
        if !matches!(self.kept, prefs::Kept::File { .. }) {
            tracing::debug!("the app's choices are kept until it quits; not written");
            return Ok(());
        }
        self.save_at(&Self::path()?)
    }

    /// [Decision Log]
    /// - Purpose: a 0600 file came back 0664 after a save (track G3's
    ///   find, 2026-11-13): `std::fs::write` creates the temporary file
    ///   with the caller's umask, and the rename puts that mode in place.
    /// - Choice: the temporary file is created new with 0600 (a stale one
    ///   from a crashed save removed first), the folder 0700 when the app
    ///   makes it: the credential store's rule
    ///   (`vessel-client/src/credential.rs`), minus its `chmod` of a
    ///   folder that already exists (the settings' folder may be the
    ///   person's, not the app's alone).
    fn save_at(&self, path: &Path) -> Result<(), LinkError> {
        use std::io::Write as _;

        let io = |error: std::io::Error| LinkError::Io(error.to_string());
        if let Some(dir) = path.parent() {
            private_dir(dir).map_err(io)?;
        }
        let text =
            serde_json::to_vec_pretty(self).map_err(|error| LinkError::Io(error.to_string()))?;
        let temporary = path.with_extension("json.tmp");
        drop(std::fs::remove_file(&temporary));
        let written = new_private_file(&temporary)
            .and_then(|mut file| file.write_all(&text))
            .and_then(|()| std::fs::rename(&temporary, path));
        if written.is_err() {
            drop(std::fs::remove_file(&temporary));
        }
        written.map_err(io)
    }
}

/// `dir` and its missing parents, made owner-only (0700) when the app
/// makes them; a folder that exists keeps its mode.
#[cfg(all(unix, not(target_arch = "wasm32")))]
pub(crate) fn private_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt as _;

    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

#[cfg(all(not(unix), not(target_arch = "wasm32")))]
pub(crate) fn private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

/// A new file at `path`, owner-only (0600) whatever the umask.
#[cfg(all(unix, not(target_arch = "wasm32")))]
pub(crate) fn new_private_file(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt as _;

    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(all(not(unix), not(target_arch = "wasm32")))]
pub(crate) fn new_private_file(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::io::Result<PathBuf> {
        let dir =
            std::env::temp_dir().join(format!("vessel-app-prefs-{}-{name}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir)?;
        Ok(dir.join("app-prefs.json"))
    }

    #[test]
    fn settings_are_kept_in_the_same_file_and_read_back() {
        let path = scratch("kept").unwrap();
        let missing = Prefs::load_at(&path);
        assert_eq!(
            missing.kept,
            prefs::Kept::File {
                path: path.display().to_string()
            }
        );
        let mut prefs = missing;
        prefs.machine = Some("gpu-box".to_owned());
        let dark: PreferenceChange =
            serde_json::from_str(r#"{"set":"theme","value":"dark"}"#).unwrap();
        prefs.prefer(&dark).unwrap();
        prefs.save_at(&path).unwrap();
        let read = Prefs::load_at(&path);
        assert_eq!(read, prefs);
        assert_eq!(read.settings.appearance.theme, prefs::Theme::Dark);
        // A file of the lead's fields only (before settings) reads as the
        // defaults for them.
        std::fs::write(&path, r#"{"machine":"laptop"}"#).unwrap();
        assert_eq!(Prefs::load_at(&path).settings, Preferences::default());
        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }

    #[test]
    fn an_unknown_language_keeps_the_file_and_its_other_preferences() {
        use vessel_app_view::localization::LanguagePreference;
        let path = scratch("language").unwrap();
        std::fs::write(&path, r#"{"machine":"laptop","last_label":"previous","settings":{"language":"removed-ZZ","appearance":{"theme":"dark"},"composer":{"send":"mod_enter"}}}"#).unwrap();
        let mut kept = Prefs::load_at(&path);
        assert!(matches!(kept.kept, prefs::Kept::File { .. }));
        assert_eq!(kept.machine.as_deref(), Some("laptop"));
        assert_eq!(kept.last_label.as_deref(), Some("previous"));
        assert_eq!(kept.settings.language, LanguagePreference::System);
        assert_eq!(kept.settings.appearance.theme, prefs::Theme::Dark);
        assert_eq!(kept.settings.composer.send, prefs::SendKey::ModEnter);
        kept.prefer(&PreferenceChange::Language(LanguagePreference::from_tag(
            "ko",
        )))
        .unwrap();
        kept.save_at(&path).unwrap();
        let read = Prefs::load_at(&path);
        assert_eq!(read, kept);
        assert_eq!(read.settings.language.tag(), "ko");
        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }

    #[test]
    fn a_file_that_cannot_be_read_is_not_written_over() {
        let path = scratch("broken").unwrap();
        std::fs::write(&path, "not json").unwrap();
        let mut prefs = Prefs::load_at(&path);
        assert!(matches!(
            &prefs.kept,
            prefs::Kept::ThisLaunch {
                why: prefs::LaunchOnly::Unreadable { .. }
            }
        ));
        prefs.machine = Some("gpu-box".to_owned());
        prefs.save().unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "not json");
        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }
    /// The file is owner-only whatever mode an older one had, the folder
    /// owner-only when the save made it, and a folder that was there
    /// keeps its own mode (G3's find: 0600 came back 0664).
    #[cfg(unix)]
    #[test]
    fn the_file_is_owner_only_and_a_new_folder_too() {
        use std::os::unix::fs::PermissionsExt as _;

        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        let root = scratch("modes").unwrap();
        let root = root.parent().unwrap();
        let path = root.join("vessel").join("app-prefs.json");
        let prefs = Prefs {
            machine: Some("gpu-box".to_owned()),
            ..Prefs::default()
        };
        prefs.save_at(&path).unwrap();
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(path.parent().unwrap()), 0o700);

        // A file left readable by an older build comes back owner-only,
        // and the folder that was there keeps its mode.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o664)).unwrap();
        std::fs::set_permissions(
            path.parent().unwrap(),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        prefs.save_at(&path).unwrap();
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(path.parent().unwrap()), 0o755);
        assert_eq!(Prefs::load_at(&path).machine.as_deref(), Some("gpu-box"));

        // A temporary file a crashed save left is replaced, not reused.
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, "half").unwrap();
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o666)).unwrap();
        prefs.save_at(&path).unwrap();
        assert_eq!(mode(&path), 0o600);
        assert!(!temporary.exists());
        drop(std::fs::remove_dir_all(root));
    }
}
