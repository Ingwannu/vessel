//! The drafts task: owns this process's saves and where drafts are kept;
//! answers reads from the file read again, merged with the saves not yet
//! written; writes at most once a second.
//!
//! [Decision Log] When a save is written
//! - Purpose: the page saves while a person types (at most every 500 ms)
//!   and at once when the tab is hidden; each save is the whole draft, up
//!   to 128 KiB, and a write is a read, a merge and a rename of up to
//!   1 MiB.
//! - Choice: a save is answered as soon as it is queued and held in
//!   memory. The first save after a quiet second is written at once,
//!   before the next op is taken; saves within the second after a write
//!   wait for its end and go out together in one read-merge-rename. A
//!   read (`drafts`, `draft`) merges the saves not yet written, so it
//!   never misses one. When `vessel ui` stops, what is held is written
//!   before it exits.
//! - Bounded: past [`MAX_DRAFTS`] keys held, the saves are written at once
//!   whatever the time, so memory holds at most that many unwritten
//!   drafts.
//! - Alternatives: a write per save (a rename of up to 1 MiB twice a
//!   second per tab); a write only on a timer (a save made just before a
//!   crash would wait for it even when nothing else was happening).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use super::disk;
use super::drafts_doc::{Decoded, DraftSet, MAX_DRAFTS, MAX_FILE, Save};
use super::{Answer, Ask, DraftsAsk, Place, blocking, unreadable};
use crate::wire::{DraftKey, DraftText, LaunchOnly, OpResult};

/// What the logs and the page call these files.
const WHAT: &str = "drafts";

/// The least time between two writes.
const WRITES: Duration = Duration::from_secs(1);

/// The task's state.
#[derive(Debug)]
pub(super) struct DraftsTask {
    place: Place,
    /// In memory for this launch: every draft. In a file: the drafts as
    /// last read or written, which memory falls back on if the file fails.
    memory: DraftSet,
    /// Saves not written yet (in a file only).
    unsaved: BTreeMap<DraftKey, Save>,
    /// The last save's time: each save is later than the one before it,
    /// even within the clock's resolution.
    last_save: Option<DateTime<Utc>>,
    /// When the last write happened.
    written: Option<Instant>,
    /// When the next write is due.
    due: Option<Instant>,
}

/// What wakes the task.
enum Wake {
    Ask(Ask<DraftsAsk>),
    Due,
    Stop,
}

impl DraftsTask {
    pub(super) fn new(place: Place) -> Self {
        Self {
            place,
            memory: DraftSet::default(),
            unsaved: BTreeMap::new(),
            last_save: None,
            written: None,
            due: None,
        }
    }

    /// Answer ops until the UI stops; then write what is held.
    pub(super) async fn run(
        mut self,
        mut queue: mpsc::Receiver<Ask<DraftsAsk>>,
        stop: CancellationToken,
    ) {
        loop {
            let due = self.due;
            let wake = tokio::select! {
                ask = queue.recv() => ask.map_or(Wake::Stop, Wake::Ask),
                () = tokio::time::sleep_until(due.unwrap_or_else(Instant::now)), if due.is_some() => Wake::Due,
                () = stop.cancelled() => Wake::Stop,
            };
            match wake {
                Wake::Ask(ask) => self.answer(ask).await,
                Wake::Due => self.write().await,
                Wake::Stop => {
                    self.write().await;
                    return;
                }
            }
        }
    }

    async fn answer(&mut self, ask: Ask<DraftsAsk>) {
        match ask.what {
            DraftsAsk::All => {
                let drafts = self.view().await.state(self.place.kept());
                let _gone = ask.answer.send(Ok(OpResult::Drafts { drafts }));
            }
            DraftsAsk::One(key) => {
                let text = self.view().await.text(&key);
                let _gone = ask.answer.send(Ok(OpResult::Draft { text }));
            }
            DraftsAsk::Save(key, text) => {
                let answer: Answer = Ok(OpResult::SaveDraft);
                let _gone = ask.answer.send(answer);
                self.save(key, text).await;
            }
        }
    }

    /// Hold a save, and write now if a second has passed since the last
    /// write (or too many are held).
    async fn save(&mut self, key: DraftKey, text: DraftText) {
        let now = Utc::now();
        let at = match self.last_save {
            Some(last) if last >= now => last
                .checked_add_signed(TimeDelta::nanoseconds(1))
                .unwrap_or(last),
            Some(_) | None => now,
        };
        self.last_save = Some(at);
        let save = Save { text, at };
        if let Place::Memory(_) = self.place {
            self.memory.merge(&BTreeMap::from([(key, save)]));
            self.memory.bound(now);
            return;
        }
        let _older = self.unsaved.insert(key, save);
        let quiet = self
            .written
            .is_none_or(|written| written.elapsed() >= WRITES);
        if quiet || self.unsaved.len() >= MAX_DRAFTS {
            self.write().await;
        } else if self.due.is_none() {
            self.due = self.written.and_then(|written| written.checked_add(WRITES));
        }
    }

    /// Every draft now: the file read again with the saves held merged
    /// in, or memory.
    async fn view(&mut self) -> DraftSet {
        let now = Utc::now();
        if let Place::File(path) = &self.place {
            match load(path).await {
                Ok(mut set) => {
                    set.merge(&self.unsaved);
                    set.bound(now);
                    self.memory = set.clone();
                    return set;
                }
                Err((why, found)) => self.keep_in_memory(why, found),
            }
        }
        let mut set = self.memory.clone();
        set.bound(now);
        set
    }

    /// Write the saves held: read, merge, rename.
    async fn write(&mut self) {
        self.due = None;
        let Place::File(path) = &self.place else {
            return;
        };
        if self.unsaved.is_empty() {
            return;
        }
        let path = path.clone();
        match load(&path).await {
            Ok(mut set) => {
                set.merge(&self.unsaved);
                set.bound(Utc::now());
                self.unsaved.clear();
                if let Err(message) = save(path.clone(), &set).await {
                    self.place = Place::Memory(unreadable(&path, WHAT, message));
                }
                self.memory = set;
            }
            Err((why, found)) => self.keep_in_memory(why, found),
        }
        self.written = Some(Instant::now());
    }

    /// Keep the drafts in memory from now on: `found` (a newer file's) or
    /// the last known, with the saves held merged in.
    fn keep_in_memory(&mut self, why: LaunchOnly, found: Option<DraftSet>) {
        let mut set = found.unwrap_or_else(|| std::mem::take(&mut self.memory));
        set.merge(&std::mem::take(&mut self.unsaved));
        set.bound(Utc::now());
        self.memory = set;
        self.place = Place::Memory(why);
        self.due = None;
    }
}

/// The drafts in the file at `path` (none when there is no file), or why
/// they are kept in memory from now on, with what a newer file held.
async fn load(path: &Path) -> Result<DraftSet, (LaunchOnly, Option<DraftSet>)> {
    let reading = path.to_owned();
    let read =
        blocking(move || disk::read(&reading, MAX_FILE).map_err(|error| error.to_string())).await;
    let bytes = match read {
        Ok(None) => return Ok(DraftSet::default()),
        Ok(Some(bytes)) => bytes,
        Err(message) => return Err((unreadable(path, WHAT, message), None)),
    };
    match DraftSet::decode(&bytes) {
        Ok(Decoded::Current(set)) => Ok(set),
        Ok(Decoded::Newer { version, set }) => {
            tracing::warn!(
                path = %path.display(),
                version,
                "the page's drafts were written by a newer vessel; they are used until vessel ui stops and not written"
            );
            let why = LaunchOnly::Newer {
                path: path.display().to_string(),
                version,
            };
            Err((why, Some(set)))
        }
        Err(error) => Err((unreadable(path, WHAT, crate::registry::chain(&error)), None)),
    }
}

/// Write `set` to `path`.
async fn save(path: PathBuf, set: &DraftSet) -> Result<(), String> {
    let bytes = set.encode().map_err(|error| error.to_string())?;
    blocking(move || disk::write(&path, &bytes).map_err(|error| error.to_string())).await
}
