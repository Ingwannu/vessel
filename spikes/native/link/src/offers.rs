//! Agents' lists kept on disk between runs (owner word 28): what each
//! agent on each node last answered to `harness.options`, and the `/`
//! commands a session of it last reported, handed to the app when the node
//! connects so a control opens on it at once
//! (`vessel_app_view::offers` decides what is shown and when to ask).
//!
//! [Decision Log]
//! - Purpose: after a restart every control opened on a spinner until the
//!   node answered, seconds when the node had to start the agent.
//! - Followed: Paseo's provider snapshot cache
//!   (`packages/app/src/data/provider-snapshot-cache.ts`): one store on the
//!   device, a format version that drops a store of another version rather
//!   than misreading it, a size bound with the oldest dropped first, keyed
//!   by the host it came from.
//! - Constraints: the file sits in the app's state folder, owner-only
//!   (0600 in a 0700 folder, as the log and the prefs are), and holds no
//!   secret: the agents' own option ids, names, descriptions and values,
//!   nothing of a node's address, token or credential. Two nodes or two
//!   books never see each other's lists: entries are keyed by the node's
//!   id (from its welcome), not by the name a book gives it, and by the
//!   agent and its profile.
//! - Alternatives: the prefs file (rewritten on every pick, and its
//!   readers would carry agents' lists they do not need); a file per node
//!   (more files to bound and to clean when a node is forgotten); keying
//!   by the book's name (a name given to another node would show this
//!   node's list).
//! - Choice: one JSON file, [`VERSION`]ed, at most [`MAX_ENTRIES`] lists
//!   and [`MAX_BYTES`] bytes, written (temporary file, then rename) only
//!   when an answer differs from the kept one.
//! - Trade-offs: a list of a node no longer in the book stays until the
//!   bound pushes it out (it is never shown: no connection hands it over).
//!   A file that does not read, or of another version, is dropped: the
//!   lists are a cache, the next answers rebuild them.

use std::io::Write as _;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vessel_proto::{AgentCommand, HarnessRef, NodeId, ProfileName, SessionOption};

/// The file's format. Another number (an older or newer build's) is not
/// read: the file is replaced by the next answer. 2: options and commands
/// each optional (1 held options only, unreleased).
pub(crate) const VERSION: u32 = 2;

/// At most this many lists (agents on nodes); the oldest go first.
pub(crate) const MAX_ENTRIES: usize = 64;

/// At most this many bytes of file; the oldest lists go first (Paseo's
/// bound is 4 MiB for whole snapshots; one agent's list is a few KiB).
pub(crate) const MAX_BYTES: usize = 1_048_576;

/// The file's name in the app's state folder.
const FILE: &str = "agent-options.json";

/// One agent's list on one node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Kept {
    /// The node's id (its welcome's), never its name in a book.
    pub(crate) node: NodeId,
    /// The agent.
    pub(crate) agent: HarnessRef,
    /// The agent's profile, when the list is a profile's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) profile: Option<ProfileName>,
    /// When it last changed.
    pub(crate) at: DateTime<Utc>,
    /// The options, as the agent sent them; `None` until it answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) options: Option<Vec<SessionOption>>,
    /// The `/` commands, as a session of the agent reported them; `None`
    /// until one did (`harness.options` carries none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) commands: Option<Vec<AgentCommand>>,
}

/// What an agent said, to keep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Said {
    /// Its options (`harness.options`).
    Options(Vec<SessionOption>),
    /// Its commands (a session's report).
    Commands(Vec<AgentCommand>),
}

#[derive(Debug, Serialize, Deserialize)]
struct Stored {
    version: u32,
    entries: Vec<Kept>,
}

/// The kept lists, and the file they are kept in.
#[derive(Debug, Default)]
pub(crate) struct Store {
    /// `None`: the platform names no state folder; nothing is written.
    path: Option<PathBuf>,
    entries: Vec<Kept>,
}

impl Store {
    /// The app's state folder's file: `$XDG_STATE_HOME/vessel` on Linux
    /// (beside the log), `%LOCALAPPDATA%\Vessel` on Windows,
    /// `~/Library/Application Support/Vessel` on macOS.
    pub(crate) fn default_path() -> Option<PathBuf> {
        let dir = if cfg!(any(windows, target_os = "macos")) {
            dirs::data_local_dir().map(|base| base.join("Vessel"))
        } else {
            dirs::state_dir()
                .or_else(dirs::data_local_dir)
                .map(|base| base.join("vessel"))
        };
        dir.map(|dir| dir.join(FILE))
    }

    /// The lists kept at `path`; none when the file is missing, does not
    /// read, or is of another version (it is replaced on the next write).
    pub(crate) fn load(path: Option<PathBuf>) -> Self {
        let entries = path
            .as_deref()
            .and_then(|path| match std::fs::read(path) {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => {
                    tracing::warn!(%error, "the kept agent lists could not be read");
                    None
                }
            })
            .and_then(|bytes| match serde_json::from_slice::<Stored>(&bytes) {
                Ok(stored) if stored.version == VERSION => Some(stored.entries),
                Ok(stored) => {
                    tracing::info!(
                        version = stored.version,
                        "kept agent lists of another version: dropped"
                    );
                    None
                }
                Err(error) => {
                    tracing::warn!(%error, "the kept agent lists do not read: dropped");
                    None
                }
            })
            .unwrap_or_default();
        let mut store = Self { path, entries };
        let _dropped = store.bound();
        store
    }

    /// The lists kept for `node` (any profile's), newest first.
    pub(crate) fn for_node(&self, node: &NodeId) -> Vec<&Kept> {
        let mut found: Vec<&Kept> = self
            .entries
            .iter()
            .filter(|kept| &kept.node == node)
            .collect();
        found.sort_by_key(|kept| std::cmp::Reverse(kept.at));
        found
    }

    /// Keep what `agent` on `node` (for `profile`) said, at `at`; `true`
    /// when that changed the kept lists (the file is then to be written).
    /// The other half of its entry (options or commands) is left as kept.
    pub(crate) fn keep(
        &mut self,
        node: NodeId,
        agent: HarnessRef,
        profile: Option<ProfileName>,
        said: Said,
        at: DateTime<Utc>,
    ) -> bool {
        let found = self
            .entries
            .iter()
            .position(|kept| kept.node == node && kept.agent == agent && kept.profile == profile);
        let index = found.unwrap_or_else(|| {
            self.entries.push(Kept {
                node,
                agent,
                profile,
                at,
                options: None,
                commands: None,
            });
            self.entries.len().saturating_sub(1)
        });
        let Some(kept) = self.entries.get_mut(index) else {
            return false;
        };
        let changed = match said {
            Said::Options(options) => replace(&mut kept.options, options),
            Said::Commands(commands) => replace(&mut kept.commands, commands),
        };
        if changed {
            kept.at = at;
            let _dropped = self.bound();
        }
        changed
    }

    /// Drop the oldest lists until at most [`MAX_ENTRIES`] and
    /// [`MAX_BYTES`] remain; how many went.
    fn bound(&mut self) -> usize {
        let before = self.entries.len();
        self.entries.sort_by_key(|kept| std::cmp::Reverse(kept.at));
        self.entries.truncate(MAX_ENTRIES);
        while !self.entries.is_empty() && self.encoded().map_or(0, |bytes| bytes.len()) > MAX_BYTES
        {
            let _oldest = self.entries.pop();
        }
        before.saturating_sub(self.entries.len())
    }

    fn encoded(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&StoredRef {
            version: VERSION,
            entries: &self.entries,
        })
    }

    /// Write the lists: a new owner-only file beside the old one, then
    /// renamed over it; the folder made owner-only when it is made.
    ///
    /// # Errors
    /// The folder or the file could not be written.
    pub(crate) fn save(&self) -> std::io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let bytes = self.encoded().map_err(std::io::Error::other)?;
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

    /// Where the file is.
    #[cfg(test)]
    fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }
}

/// Put `new` in `slot` unless it holds the same; whether it changed.
fn replace<T: PartialEq>(slot: &mut Option<T>, new: T) -> bool {
    if slot.as_ref() == Some(&new) {
        return false;
    }
    *slot = Some(new);
    true
}

/// [`Stored`] by reference, for writing without copying the lists.
#[derive(Serialize)]
struct StoredRef<'a> {
    version: u32,
    entries: &'a [Kept],
}

#[cfg(test)]
mod tests;

/// Keep an agent's answer on this device, written only when it differs
/// from the kept one; a write that fails is logged (the list is a cache).
pub(crate) async fn keep_answer(
    store: std::sync::Arc<std::sync::Mutex<Store>>,
    node: NodeId,
    harness: HarnessRef,
    said: Said,
    at: DateTime<Utc>,
) {
    let kept = tokio::task::spawn_blocking(move || {
        let mut store = store
            .lock()
            .map_err(|_poisoned| std::io::Error::other("the kept lists' lock is poisoned"))?;
        if store.keep(node, harness, None, said, at) {
            store.save()
        } else {
            Ok(())
        }
    })
    .await;
    match kept {
        Ok(Ok(())) => {}
        Ok(Err(error)) => tracing::warn!(%error, "an agent's list could not be kept"),
        Err(error) => tracing::warn!(%error, "an agent's list was not kept"),
    }
}
