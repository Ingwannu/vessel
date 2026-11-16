//! One canonical preference actor applies typed deltas and broadcasts committed snapshots.

mod merge;
mod scrollback;
mod writer;

pub use scrollback::{CommittedScrollback, ScrollbackBatch};

use super::DesktopError;
use crate::Prefs;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use tokio::sync::{oneshot, watch};
use vessel_app_view::prefs::Kept;

const QUEUE_CAPACITY: usize = 64;

/// Revisions order outcomes as well as successful saves, so stale failures cannot win.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Revision(u64);

/// Launch-only choices remain useful without pretending that a file was written.
#[derive(Debug, Clone)]
pub enum Outcome {
    /// The existing preferences file was committed.
    Saved,
    /// The initial file was unreadable; only memory was changed.
    LaunchOnly,
    /// The canonical snapshot is unchanged after a refused edit.
    Failed(Arc<DesktopError>),
}

/// The file and every window observe the same canonical successful snapshot.
#[derive(Debug, Clone)]
pub struct Snapshot {
    revision: Revision,
    prefs: Arc<Prefs>,
    outcome: Outcome,
    scrollback: Option<CommittedScrollback>,
}

impl Snapshot {
    /// Revision includes failures and is monotonically ordered.
    #[must_use]
    pub const fn revision(&self) -> Revision {
        self.revision
    }
    /// Clone the Arc rather than every map when fanning out to windows.
    #[must_use]
    pub fn preferences(&self) -> Arc<Prefs> {
        Arc::clone(&self.prefs)
    }
    /// Each window can translate a failure without receiving file contents.
    #[must_use]
    pub const fn outcome(&self) -> &Outcome {
        &self.outcome
    }
    /// Saved-only terminal state survives failed or launch-only projections.
    #[must_use]
    pub const fn committed_scrollback(&self) -> Option<CommittedScrollback> {
        self.scrollback
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Sequence(u64);
#[derive(Debug, Clone, Copy, Default)]
struct Acknowledged {
    sequence: Sequence,
    revision: Revision,
}
#[derive(Debug)]
struct Pending {
    sequence: Sequence,
    before: Arc<Prefs>,
    after: Arc<Prefs>,
}

/// Rebased local intent remains visible until its own correlated acknowledgement.
#[derive(Debug)]
pub enum Observation {
    /// This scheduled snapshot predates one already applied by the window.
    Stale,
    /// Canonical state with this window's still-unacknowledged edits rebased on it.
    Applied(Arc<Prefs>),
    /// The combined local projection exceeds the same model bound as persistence.
    Refused {
        /// Valid projection with only admitted pending edits applied.
        prefs: Arc<Prefs>,
        /// The existing preference model explains why an edit was refused.
        failure: vessel_app_view::prefs::Refused,
    },
}

/// One accepted write retains its exact outcome even if watch updates coalesce.
#[derive(Debug)]
pub struct Receipt(oneshot::Receiver<Snapshot>);

#[derive(Debug, Clone)]
struct Publication(Arc<()>);

/// A completed queue checkpoint with an opaque accepted-publication identity.
/// Lifecycle admission rechecks it on the same UI executor as its Clients.
#[derive(Debug)]
pub struct Checkpoint {
    publication: Publication,
    snapshot: Snapshot,
}

impl Checkpoint {
    /// Preserve launch-only and failed-write outcomes rather than claiming saved.
    #[must_use]
    pub const fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
}

impl Receipt {
    /// Await this request, not an unrelated later write. Observe this snapshot
    /// through Client before applying; a newer revision still wins.
    ///
    /// # Errors
    /// A stopped writer cannot acknowledge an accepted request.
    pub async fn completed(self) -> Result<Snapshot, DesktopError> {
        self.0.await.map_err(|_closed| DesktopError::Closed)
    }
}

enum Command {
    Change {
        before: Arc<Prefs>,
        after: Arc<Prefs>,
        sequence: Sequence,
        acknowledged: watch::Sender<Acknowledged>,
        receipt: Option<oneshot::Sender<Snapshot>>,
    },
    Flush(oneshot::Sender<()>),
}

/// Process-wide handle; no per-window writer can race the atomic file helper.
#[derive(Debug)]
pub struct Coordinator {
    commands: mpsc::SyncSender<Command>,
    current: watch::Receiver<Snapshot>,
    publication: watch::Sender<Publication>,
    scrollback: scrollback::Hub,
}

/// UI-thread client tracks its own optimistic baseline between acknowledgements.
#[derive(Debug)]
pub struct Client {
    commands: mpsc::SyncSender<Command>,
    baseline: RefCell<Arc<Prefs>>,
    canonical: RefCell<Arc<Prefs>>,
    revision: std::cell::Cell<Revision>,
    sequence: std::cell::Cell<Sequence>,
    pending: RefCell<VecDeque<Pending>>,
    acknowledged: watch::Sender<Acknowledged>,
    acknowledgements: watch::Receiver<Acknowledged>,
    refused: std::cell::Cell<Option<crate::Rejected>>,
    publication: watch::Sender<Publication>,
    scrollback: scrollback::Subscription,
    applied_scrollback: std::cell::Cell<Option<CommittedScrollback>>,
}

impl Coordinator {
    /// Preserve the loaded path and launch-only refusal exactly as Prefs read them.
    ///
    /// # Errors
    /// Refuses a relative persisted path or a worker that cannot start.
    pub fn start(initial: Prefs) -> Result<Self, DesktopError> {
        let path = match &initial.kept {
            Kept::File { path } => {
                let path = PathBuf::from(path);
                if !path.is_absolute() {
                    return Err(DesktopError::RelativePath);
                }
                Some(path)
            }
            Kept::ThisLaunch { .. } => None,
        };
        let outcome = if path.is_some() {
            Outcome::Saved
        } else {
            Outcome::LaunchOnly
        };
        let scrollback = path.as_ref().map(|_| {
            CommittedScrollback::new(initial.settings.terminal_scrollback, Revision::default())
        });
        let snapshot = Snapshot {
            revision: Revision::default(),
            prefs: Arc::new(initial),
            outcome,
            scrollback,
        };
        let (published, current) = watch::channel(snapshot.clone());
        let (commands, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let scrollback = scrollback::Hub::default();
        let delivery = scrollback.clone();
        let _worker = std::thread::Builder::new()
            .name("vessel-preferences".into())
            .spawn(move || {
                writer::run(snapshot, path.as_deref(), &published, &receiver, &delivery)
            })?;
        let (publication, _initial) = watch::channel(Publication(Arc::new(())));
        Ok(Self {
            commands,
            current,
            publication,
            scrollback,
        })
    }

    /// A new window starts from the latest committed snapshot, including launch-only edits.
    #[must_use]
    pub fn client(&self) -> (Client, watch::Receiver<Snapshot>) {
        let (snapshot, scrollback) = self.scrollback.subscribe(&self.current);
        let (acknowledged, acknowledgements) = watch::channel(Acknowledged::default());
        (
            Client {
                commands: self.commands.clone(),
                baseline: RefCell::new(snapshot.preferences()),
                canonical: RefCell::new(snapshot.preferences()),
                revision: std::cell::Cell::new(snapshot.revision()),
                sequence: std::cell::Cell::new(Sequence::default()),
                pending: RefCell::new(VecDeque::new()),
                acknowledged,
                acknowledgements,
                refused: std::cell::Cell::new(None),
                publication: self.publication.clone(),
                scrollback,
                applied_scrollback: std::cell::Cell::new(None),
            },
            self.current.clone(),
        )
    }

    /// Window creation reads memory, not an older disk snapshot.
    #[must_use]
    pub fn current(&self) -> Arc<Prefs> {
        self.current.borrow().preferences()
    }

    /// Rejection recovery needs the same revision and outcome as the canonical value.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        self.current.borrow().clone()
    }

    /// Capture before awaiting; a later accepted edit invalidates this receipt
    /// even if its disk write has not published a newer canonical revision yet.
    ///
    /// # Errors
    /// Returns the same worker/spawn failures as flush.
    pub fn checkpoint(
        &self,
    ) -> impl Future<Output = Result<Checkpoint, DesktopError>> + 'static + use<> {
        let publication = self.publication.borrow().clone();
        let current = self.current.clone();
        let flush = self.flush();
        async move {
            flush.await?;
            let snapshot = current.borrow().clone();
            Ok(Checkpoint {
                publication,
                snapshot,
            })
        }
    }

    /// Called immediately before quit/restart, without a subsequent await and
    /// on the same UI executor that publishes through this coordinator's Clients.
    #[must_use]
    pub fn is_current(&self, checkpoint: &Checkpoint) -> bool {
        self.scrollback.healthy()
            && Arc::ptr_eq(&self.publication.borrow().0, &checkpoint.publication.0)
            && self.current.borrow().revision() == checkpoint.snapshot.revision()
    }

    /// Wait for every accepted edit before the process exits.
    ///
    /// # Errors
    /// Returns worker startup or closure failures; save errors remain in outcomes.
    pub fn flush(&self) -> impl Future<Output = Result<(), DesktopError>> + 'static + use<> {
        let commands = self.commands.clone();
        let delivery = self.scrollback.clone();
        async move {
            if !delivery.healthy() {
                return Err(DesktopError::Closed);
            }
            let (done, wait) = oneshot::channel();
            let _sender = std::thread::Builder::new()
                .name("vessel-preferences-flush".into())
                .spawn(move || drop(commands.send(Command::Flush(done))))?;
            wait.await.map_err(|_closed| DesktopError::Closed)
        }
    }
}

impl Client {
    /// Drain once before applying to the current owner, outside every hub lock.
    /// The caller must invoke its model callbacks synchronously without awaiting.
    ///
    /// # Errors
    /// Unavailable delivery refuses effects and must refuse owner replacement.
    pub fn drain_scrollback(&self) -> Result<Option<ScrollbackBatch>, DesktopError> {
        let drained = self.scrollback.drain()?;
        self.applied_scrollback.set(drained.latest);
        Ok(drained.batch)
    }

    /// Reuse this exact UI-applied value for a successor after the drain; do not
    /// perform a separate newer global read across the ownership boundary.
    #[must_use]
    pub fn applied_scrollback(&self) -> vessel_app_view::terminal::ScrollbackLines {
        self.applied_scrollback.get().map_or(
            vessel_app_view::terminal::ScrollbackLines::DEFAULT,
            CommittedScrollback::limit,
        )
    }

    /// The last observed canonical value excludes this window's optimistic edits.
    #[must_use]
    pub fn canonical_preferences(&self) -> Arc<Prefs> {
        Arc::clone(&self.canonical.borrow())
    }

    /// Publish only changes since this window's last observed/submitted snapshot.
    ///
    /// # Errors
    /// A full or closed queue leaves the baseline unchanged. Observe canonical
    /// state before another attempt so refused fields cannot leak into a later edit.
    pub fn publish(&self, prefs: &Prefs) -> Result<(), crate::Rejected> {
        self.enqueue(prefs, None)
    }

    /// The same writer and merge path, with an exact per-request acknowledgment.
    ///
    /// # Errors
    /// Queue refusal leaves the optimistic baseline unchanged and requires
    /// canonical observation before a later write can be admitted.
    pub fn publish_with_receipt(&self, prefs: &Prefs) -> Result<Receipt, crate::Rejected> {
        let (done, receipt) = oneshot::channel();
        self.enqueue(prefs, Some(done))?;
        Ok(Receipt(receipt))
    }

    fn enqueue(
        &self,
        prefs: &Prefs,
        receipt: Option<oneshot::Sender<Snapshot>>,
    ) -> Result<(), crate::Rejected> {
        if !self.scrollback.healthy() {
            self.refused.set(Some(crate::Rejected::Closed));
            return Err(crate::Rejected::Closed);
        }
        if let Some(why) = self.refused.get() {
            return Err(why);
        }
        if self.pending.borrow().len() >= QUEUE_CAPACITY {
            self.refused.set(Some(crate::Rejected::Full));
            return Err(crate::Rejected::Full);
        }
        let sequence = Sequence(
            self.sequence
                .get()
                .0
                .checked_add(1)
                .ok_or(crate::Rejected::Closed)?,
        );
        let after = Arc::new(prefs.clone());
        let before = Arc::clone(&self.baseline.borrow());
        let sent = self
            .commands
            .try_send(Command::Change {
                before: Arc::clone(&before),
                after: Arc::clone(&after),
                sequence,
                acknowledged: self.acknowledged.clone(),
                receipt,
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => crate::Rejected::Full,
                mpsc::TrySendError::Disconnected(_) => crate::Rejected::Closed,
            });
        if let Err(why) = sent {
            // A rejected full snapshot still exists in the caller's UI. Until
            // canonical observation replaces it, a later snapshot could retry
            // its refused fields as if they were a fresh unrelated edit.
            self.refused.set(Some(why));
            return Err(why);
        }
        self.sequence.set(sequence);
        self.pending.borrow_mut().push_back(Pending {
            sequence,
            before,
            after: Arc::clone(&after),
        });
        let _previous = self.baseline.replace(after);
        let _previous = self.publication.send_replace(Publication(Arc::new(())));
        Ok(())
    }

    /// A live window must reconcile its optimistic state/refusals before exit.
    /// An accepted write's acknowledgment alone does not update its UI projection.
    #[must_use]
    pub fn is_settled(&self, checkpoint: &Checkpoint) -> bool {
        let acknowledged = *self.acknowledgements.borrow();
        self.refused.get().is_none()
            && self.pending.borrow().is_empty()
            // Another window can commit while this one has no local writes.
            // The shared publication identity alone cannot prove its watch was
            // observed and its canonical projection was applied.
            && self.revision.get() == checkpoint.snapshot.revision()
            && acknowledged.sequence == self.sequence.get()
            && acknowledged.revision <= checkpoint.snapshot.revision()
            && Arc::ptr_eq(&self.publication.borrow().0, &checkpoint.publication.0)
            && self.scrollback.settled(self.applied_scrollback.get())
    }

    /// Rebase pending intent; an old failure cannot roll back a newer committed edit.
    #[must_use]
    pub fn observe(&self, snapshot: &Snapshot) -> Observation {
        if snapshot.revision < self.revision.get() {
            return Observation::Stale;
        }
        self.revision.set(snapshot.revision);
        let _previous = self.canonical.replace(snapshot.preferences());
        let acknowledged = *self.acknowledgements.borrow();
        let mut pending = self.pending.borrow_mut();
        // A later acknowledgement may race an earlier watch delivery. Only a
        // revision included by this snapshot can remove an optimistic edit.
        if acknowledged.revision <= snapshot.revision {
            pending.retain(|edit| edit.sequence > acknowledged.sequence);
        }
        let mut projection = (*snapshot.prefs).clone();
        let mut refused = None;
        for edit in pending.iter() {
            match merge::apply(&projection, &edit.before, &edit.after) {
                Ok(next) => projection = next,
                Err(failure) => refused = Some(failure),
            }
        }
        let prefs = Arc::new(projection);
        let _previous = self.baseline.replace(Arc::clone(&prefs));
        self.refused.set(None);
        match refused {
            Some(failure) => Observation::Refused { prefs, failure },
            None => Observation::Applied(prefs),
        }
    }
}

#[cfg(test)]
mod tests;
