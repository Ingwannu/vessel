//! The preferences task: owns the document and where it is kept, applies
//! changes one at a time, and publishes every new state.
//!
//! Each op reads the file again first (another process may have changed
//! it); a state that differs from the published one is published with
//! the next revision, so every tab sees a change found in the file too. A
//! change is checked against the bounds, written (temp and rename), then
//! published; its answer is the state after it.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;

use super::disk;
use super::prefs_doc::{Decoded, MAX_FILE, PrefsDoc};
use super::{Answer, Ask, Place, PrefsAsk, blocking, unreadable};
use crate::ops::OpError;
use crate::wire::{LaunchOnly, OpResult, PreferenceChange, PreferencesState, Revision};

/// What the logs and the page call these files.
const WHAT: &str = "preferences";

/// The task's state.
#[derive(Debug)]
pub(super) struct PrefsTask {
    place: Place,
    doc: PrefsDoc,
    published: watch::Sender<Arc<PreferencesState>>,
}

impl PrefsTask {
    /// Read where the preferences are kept and publish the launch's first
    /// state.
    pub(super) async fn start(place: Place) -> (Self, watch::Receiver<Arc<PreferencesState>>) {
        let (place, doc) = load(place, PrefsDoc::default()).await;
        let first = PreferencesState {
            revision: Revision::FIRST,
            kept: place.kept(),
            values: doc.values(),
        };
        let (published, states) = watch::channel(Arc::new(first));
        (
            Self {
                place,
                doc,
                published,
            },
            states,
        )
    }

    /// Answer ops until the UI stops.
    pub(super) async fn run(
        mut self,
        mut queue: mpsc::Receiver<Ask<PrefsAsk>>,
        stop: CancellationToken,
    ) {
        loop {
            let ask = tokio::select! {
                ask = queue.recv() => ask,
                () = stop.cancelled() => None,
            };
            let Some(ask) = ask else {
                return;
            };
            let answer = match ask.what {
                PrefsAsk::Read => self.read().await,
                PrefsAsk::Change(change) => self.change(&change).await,
            };
            let _gone = ask.answer.send(answer);
        }
    }

    async fn read(&mut self) -> Answer {
        self.refresh().await;
        Ok(OpResult::Preferences {
            preferences: self.current(),
        })
    }

    async fn change(&mut self, change: &PreferenceChange) -> Answer {
        self.refresh().await;
        let next = self
            .doc
            .changed(change)
            .map_err(|refused| OpError::failed(refused.to_string()))?;
        if next != self.doc {
            if let Place::File(path) = &self.place
                && let Err(message) = save(path.clone(), &next).await
            {
                self.place = Place::Memory(unreadable(path, WHAT, message));
            }
            self.doc = next;
            self.publish();
        }
        Ok(OpResult::Prefer {
            preferences: self.current(),
        })
    }

    /// Read the file again; publish what changed.
    async fn refresh(&mut self) {
        let place = self.place.clone();
        let doc = std::mem::take(&mut self.doc);
        (self.place, self.doc) = load(place, doc).await;
        self.publish();
    }

    /// Publish the state, if it differs from the one published.
    fn publish(&self) {
        let kept = self.place.kept();
        let values = self.doc.values();
        let next = {
            let now = self.published.borrow();
            (now.kept != kept || now.values != values).then(|| now.revision.next())
        };
        if let Some(revision) = next {
            let _old = self.published.send_replace(Arc::new(PreferencesState {
                revision,
                kept,
                values,
            }));
        }
    }

    fn current(&self) -> Arc<PreferencesState> {
        Arc::clone(&self.published.borrow())
    }
}

/// The document where `place` keeps it, and where it is kept after the
/// read: a file that cannot be read moves it to memory, keeping `current`
/// (or, for a newer file, what could be read of it).
async fn load(place: Place, current: PrefsDoc) -> (Place, PrefsDoc) {
    let Place::File(path) = place else {
        return (place, current);
    };
    let reading = path.clone();
    let read =
        blocking(move || disk::read(&reading, MAX_FILE).map_err(|error| error.to_string())).await;
    let bytes = match read {
        Ok(None) => return (Place::File(path), PrefsDoc::default()),
        Ok(Some(bytes)) => bytes,
        Err(message) => return (Place::Memory(unreadable(&path, WHAT, message)), current),
    };
    match PrefsDoc::decode(&bytes) {
        Ok(Decoded::Current(doc)) => (Place::File(path), doc),
        Ok(Decoded::Newer { version, doc }) => {
            tracing::warn!(
                path = %path.display(),
                version,
                "the page's preferences were written by a newer vessel; they are used until vessel ui stops and not written"
            );
            let why = LaunchOnly::Newer {
                path: path.display().to_string(),
                version,
            };
            (Place::Memory(why), doc)
        }
        Err(error) => {
            let message = crate::registry::chain(&error);
            (Place::Memory(unreadable(&path, WHAT, message)), current)
        }
    }
}

/// Write `doc` to `path`.
async fn save(path: PathBuf, doc: &PrefsDoc) -> Result<(), String> {
    let bytes = doc.encode().map_err(|error| error.to_string())?;
    blocking(move || disk::write(&path, &bytes).map_err(|error| error.to_string())).await
}
