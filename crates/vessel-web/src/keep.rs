//! What the bridge keeps for the page across launches (ADR 0025): its
//! preferences and its drafts, each owned by one task.
//!
//! [Decision Log]
//! - Purpose: the page's origin is new on every launch (ADR 0017), so the
//!   browser forgets everything the page stored; the bridge is the page's
//!   only memory. Several tabs, and several `vessel ui` processes, share
//!   it.
//! - Choice: one task per file ([`Keeper::start`]), owning that file's
//!   state; tabs hold a [`Keeper`], a cheap handle (two bounded queues and
//!   a `watch`). A tab hands its ops to a queue from its read loop, in the
//!   order its frames arrive (`bridge`), so two changes from one tab apply
//!   in order, and a full queue answers `busy` rather than making the tab
//!   wait. Each change reads the file right before it and renames over it
//!   after, so a change another process made in between is kept.
//! - Every state of the preferences is published on a `watch` with a
//!   rising [`Revision`]: tabs push the latest one (skipping what they
//!   would overwrite anyway), and the index's first paint reads it from
//!   memory, never from the disk (`paint`).
//! - Where: the paths come from the [`NodeSource`](crate::NodeSource),
//!   asked once at start ([`PageFiles`]); this crate never works out a
//!   home directory itself (AGENTS.md section 8). No path, an error, an
//!   unreadable or a newer file, or a failure later in the launch: kept in
//!   memory for this launch ([`LaunchOnly`]), and the page says so.
//! - Alternatives: the ops as tasks spawned per request (two changes from
//!   one tab could be applied out of order, the first review's finding);
//!   one task for both files (a slow drafts write would hold up a theme
//!   change); a lock around shared state (AGENTS.md section 5).
//! - Must not: read the file per request outside these tasks, write a file
//!   it could not read, or let a tab's ops take the permits of its node
//!   calls (a draft saved as the page is hidden must not be refused because
//!   calls are in flight).

mod disk;
mod drafts_doc;
mod drafts_task;
mod lenient;
mod prefs_doc;
mod prefs_task;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::ops::OpError;
use crate::source::PageFiles;
use crate::wire::{
    Appearance, DraftKey, DraftText, Kept, LaunchOnly, OpResult, PreferenceChange, PreferencesState,
};

/// Ops waiting for one task; past it, `busy`.
pub(crate) const QUEUE: usize = 64;

/// An op's answer.
pub(crate) type Answer = Result<OpResult, OpError>;

/// One op for a task, and where its answer goes.
#[derive(Debug)]
pub(crate) struct Ask<A> {
    what: A,
    answer: oneshot::Sender<Answer>,
}

/// What the preferences task is asked.
#[derive(Debug)]
pub(crate) enum PrefsAsk {
    /// The preferences, read again.
    Read,
    /// One change.
    Change(PreferenceChange),
}

/// What the drafts task is asked.
#[derive(Debug)]
pub(crate) enum DraftsAsk {
    /// Every draft, read again.
    All,
    /// One draft, read again.
    One(DraftKey),
    /// Keep a draft (empty: remove it).
    Save(DraftKey, DraftText),
}

/// An op on what the bridge keeps for the page.
#[derive(Debug)]
pub(crate) enum KeptOp {
    /// The preferences'.
    Preferences(PrefsAsk),
    /// The drafts'.
    Drafts(DraftsAsk),
}

/// Where one file is kept now.
#[derive(Debug, Clone)]
enum Place {
    /// In this file.
    File(PathBuf),
    /// In memory, for this launch.
    Memory(LaunchOnly),
}

impl Place {
    /// The place for `path`, as the source answered.
    fn new(path: Option<PathBuf>, unknown: Option<&str>) -> Self {
        match (path, unknown) {
            (_, Some(message)) => Self::Memory(LaunchOnly::NoPath {
                message: message.to_owned(),
            }),
            (Some(path), None) => Self::File(path),
            (None, None) => Self::Memory(LaunchOnly::NoPlace),
        }
    }

    /// What the page is told.
    fn kept(&self) -> Kept {
        match self {
            Self::File(path) => Kept::File {
                path: path.display().to_string(),
            },
            Self::Memory(why) => Kept::ThisLaunch { why: why.clone() },
        }
    }
}

/// Storage moved to memory for this launch because `path` could not be
/// read or written: said once, in the log, and to the page.
fn unreadable(path: &Path, what: &'static str, message: String) -> LaunchOnly {
    tracing::warn!(
        path = %path.display(),
        %message,
        "the page's {what} cannot be kept in their file; they are kept until vessel ui stops"
    );
    LaunchOnly::Unreadable {
        path: path.display().to_string(),
        message,
    }
}

/// Run blocking file work off the async workers; a task that panicked is
/// an error like any other.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .unwrap_or_else(|error| Err(error.to_string()))
}

/// The handle tabs and the server hold.
#[derive(Debug, Clone)]
pub(crate) struct Keeper {
    preferences: mpsc::Sender<Ask<PrefsAsk>>,
    drafts: mpsc::Sender<Ask<DraftsAsk>>,
    state: watch::Receiver<Arc<PreferencesState>>,
}

/// The two tasks, to stop and wait for when the UI stops (the drafts'
/// last saves are written then).
#[derive(Debug)]
pub(crate) struct KeeperTasks {
    stop: CancellationToken,
    tasks: [JoinHandle<()>; 2],
}

impl KeeperTasks {
    /// Stop both tasks and wait until they have.
    pub(crate) async fn finish(self) {
        self.stop.cancel();
        for task in self.tasks {
            if let Err(error) = task.await {
                tracing::warn!(%error, "a task keeping the page's files ended badly");
            }
        }
    }
}

impl Keeper {
    /// Start the tasks for the files `answer` names (or why there are
    /// none), reading the preferences once before the first page is
    /// served. They stop with `stop`.
    pub(crate) async fn start(
        answer: Result<PageFiles, String>,
        stop: CancellationToken,
    ) -> (Self, KeeperTasks) {
        let (files, unknown) = match answer {
            Ok(files) => (files, None),
            Err(message) => (PageFiles::default(), Some(message)),
        };
        let preferences = Place::new(files.preferences, unknown.as_deref());
        let drafts = Place::new(files.drafts, unknown.as_deref());
        let (prefs, state) = prefs_task::PrefsTask::start(preferences).await;
        let (prefs_queue, prefs_asks) = mpsc::channel(QUEUE);
        let (drafts_queue, drafts_asks) = mpsc::channel(QUEUE);
        let tasks = [
            tokio::spawn(prefs.run(prefs_asks, stop.clone())),
            tokio::spawn(drafts_task::DraftsTask::new(drafts).run(drafts_asks, stop.clone())),
        ];
        let keeper = Self {
            preferences: prefs_queue,
            drafts: drafts_queue,
            state,
        };
        (keeper, KeeperTasks { stop, tasks })
    }

    /// Hand `op` to its task; the answer comes on the receiver. A full
    /// queue answers `busy` at once, without waiting.
    pub(crate) fn ask(&self, op: KeptOp) -> oneshot::Receiver<Answer> {
        let (answer, reply) = oneshot::channel();
        match op {
            KeptOp::Preferences(what) => offer(&self.preferences, Ask { what, answer }),
            KeptOp::Drafts(what) => offer(&self.drafts, Ask { what, answer }),
        }
        reply
    }

    /// The preferences' states, from the current one on.
    pub(crate) fn states(&self) -> watch::Receiver<Arc<PreferencesState>> {
        self.state.clone()
    }

    /// How the page looks now, from memory.
    pub(crate) fn appearance(&self) -> Appearance {
        self.state.borrow().values.appearance
    }
}

fn offer<A>(queue: &mpsc::Sender<Ask<A>>, ask: Ask<A>) {
    match queue.try_send(ask) {
        Ok(()) => {}
        Err(TrySendError::Full(ask)) => {
            let _gone = ask.answer.send(Err(OpError::busy()));
        }
        Err(TrySendError::Closed(ask)) => {
            let _gone = ask
                .answer
                .send(Err(OpError::failed("the UI is stopping".to_owned())));
        }
    }
}
