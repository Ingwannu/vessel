//! What was typed in a composer and not sent, kept on the device between
//! runs (`vessel_app_view::composer::drafts` decides what is kept and its
//! bounds): one owner-only file beside the agents' lists.
//!
//! [Decision Log]
//! - Purpose: a draft typed in a session, or in the new chat, came back
//!   empty after switching sessions or restarting (track P's row e).
//! - Followed: Paseo keeps drafts on the device (`stores/draft-store`,
//!   persisted storage); the page keeps them per session in its bridge's
//!   document with the same bounds as `Drafts`.
//! - Constraints: drafts are the person's own words, so the file is
//!   owner-only (0600 in a 0700 folder) and never logged; read once at
//!   start (the composers are filled on their first frame), written on the
//!   blocking pool, a second after the last edit (the screen's debounce).
//! - Alternatives: the prefs file (rewritten on every pick, and its
//!   readers would carry the person's text); the agents' lists' file (a
//!   cache that may be dropped at any time, drafts may not).
//! - Choice: `drafts.json` in the app's state folder, versioned: a file of
//!   another version, or one that does not read, is set aside as
//!   `drafts.json.unread` (never overwritten by the next write: the text is
//!   the person's) and the app starts with none.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use vessel_app_view::composer::drafts::Drafts;

/// The file's format.
pub(crate) const VERSION: u32 = 1;

/// The file's name in the app's state folder.
const FILE: &str = "drafts.json";

#[derive(Debug, Serialize, Deserialize)]
struct Stored {
    version: u32,
    drafts: Drafts,
}

#[derive(Serialize)]
struct StoredRef<'a> {
    version: u32,
    drafts: &'a Drafts,
}

/// Where the drafts are kept: beside the agents' lists.
pub(crate) fn default_path() -> Option<PathBuf> {
    crate::offers::Store::default_path().and_then(|lists| lists.parent().map(|dir| dir.join(FILE)))
}

/// The drafts kept on this device; none when the file is missing. A file
/// that does not read, or of another version, is moved aside (kept, not
/// overwritten) and none are returned.
#[must_use]
pub fn load() -> Drafts {
    default_path().map_or_else(Drafts::default, |path| load_at(&path))
}

pub(crate) fn load_at(path: &Path) -> Drafts {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Drafts::default(),
        Err(error) => {
            tracing::warn!(%error, "the kept drafts could not be read");
            return Drafts::default();
        }
    };
    match serde_json::from_slice::<Stored>(&bytes) {
        Ok(stored) if stored.version == VERSION => {
            let mut drafts = stored.drafts;
            drafts.prune(Utc::now());
            drafts
        }
        Ok(_) | Err(_) => {
            // The text is the person's: set aside, not dropped.
            let aside = path.with_extension("json.unread");
            if let Err(error) = std::fs::rename(path, &aside) {
                tracing::warn!(%error, "the kept drafts do not read and could not be set aside");
            } else {
                tracing::warn!("the kept drafts do not read: set aside");
            }
            Drafts::default()
        }
    }
}

/// Write the drafts: a new owner-only file beside the old one, renamed
/// over it; the folder made owner-only when it is made.
///
/// # Errors
/// The folder or the file could not be written.
pub(crate) fn save_at(path: &Path, drafts: &Drafts) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(&StoredRef {
        version: VERSION,
        drafts,
    })
    .map_err(std::io::Error::other)?;
    if let Some(dir) = path.parent() {
        crate::prefs::private_dir(dir)?;
    }
    let temporary = path.with_extension("json.tmp");
    drop(std::fs::remove_file(&temporary));
    let written = crate::prefs::new_private_file(&temporary)
        .and_then(|mut file| file.write_all(&bytes))
        .and_then(|()| std::fs::rename(&temporary, path));
    if written.is_err() {
        drop(std::fs::remove_file(&temporary));
    }
    written
}

/// Write the drafts where they are kept (nothing where the platform names
/// no state folder).
pub(crate) fn save(drafts: &Drafts) -> std::io::Result<()> {
    match default_path() {
        Some(path) => save_at(&path, drafts),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests;

/// Write the composers' drafts on the blocking pool; a failure is logged
/// (never their text).
pub(crate) fn save_in_background(drafts: Drafts) {
    drop(tokio::task::spawn_blocking(move || {
        if let Err(error) = save(&drafts) {
            tracing::warn!(%error, "the drafts could not be kept");
        }
    }));
}
