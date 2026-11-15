//! One canonical preference actor applies typed deltas and broadcasts committed snapshots.

mod merge;

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

enum Command {
    Change {
        before: Arc<Prefs>,
        after: Arc<Prefs>,
        sequence: Sequence,
        acknowledged: watch::Sender<Acknowledged>,
    },
    Flush(oneshot::Sender<()>),
}

/// Process-wide handle; no per-window writer can race the atomic file helper.
#[derive(Debug)]
pub struct Coordinator {
    commands: mpsc::SyncSender<Command>,
    current: watch::Receiver<Snapshot>,
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
        let snapshot = Snapshot {
            revision: Revision::default(),
            prefs: Arc::new(initial),
            outcome,
        };
        let (published, current) = watch::channel(snapshot.clone());
        let (commands, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let _worker = std::thread::Builder::new()
            .name("vessel-preferences".into())
            .spawn(move || run(snapshot, path.as_deref(), &published, &receiver))?;
        Ok(Self { commands, current })
    }

    /// A new window starts from the latest committed snapshot, including launch-only edits.
    #[must_use]
    pub fn client(&self) -> (Client, watch::Receiver<Snapshot>) {
        let snapshot = self.current.borrow().clone();
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

    /// Wait for every accepted edit before the process exits.
    ///
    /// # Errors
    /// Returns worker startup or closure failures; save errors remain in outcomes.
    pub fn flush(&self) -> impl Future<Output = Result<(), DesktopError>> + 'static + use<> {
        let commands = self.commands.clone();
        async move {
            let (done, wait) = oneshot::channel();
            let _sender = std::thread::Builder::new()
                .name("vessel-preferences-flush".into())
                .spawn(move || drop(commands.send(Command::Flush(done))))?;
            wait.await.map_err(|_closed| DesktopError::Closed)
        }
    }
}

impl Client {
    /// The last observed canonical value excludes this window's optimistic edits.
    #[must_use]
    pub fn canonical_preferences(&self) -> Arc<Prefs> {
        Arc::clone(&self.canonical.borrow())
    }

    /// Publish only changes since this window's last observed/submitted snapshot.
    ///
    /// # Errors
    /// A full or closed queue leaves the baseline unchanged for an explicit retry.
    pub fn publish(&self, prefs: &Prefs) -> Result<(), crate::Rejected> {
        if self.pending.borrow().len() >= QUEUE_CAPACITY {
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
        self.commands
            .try_send(Command::Change {
                before: Arc::clone(&before),
                after: Arc::clone(&after),
                sequence,
                acknowledged: self.acknowledged.clone(),
            })
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => crate::Rejected::Full,
                mpsc::TrySendError::Disconnected(_) => crate::Rejected::Closed,
            })?;
        self.sequence.set(sequence);
        self.pending.borrow_mut().push_back(Pending {
            sequence,
            before,
            after: Arc::clone(&after),
        });
        let _previous = self.baseline.replace(after);
        Ok(())
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
        match refused {
            Some(failure) => Observation::Refused { prefs, failure },
            None => Observation::Applied(prefs),
        }
    }
}

fn run(
    mut snapshot: Snapshot,
    path: Option<&std::path::Path>,
    published: &watch::Sender<Snapshot>,
    receiver: &mpsc::Receiver<Command>,
) {
    while let Ok(command) = receiver.recv() {
        match command {
            Command::Flush(done) => drop(done.send(())),
            Command::Change {
                before,
                after,
                sequence,
                acknowledged,
            } => {
                let Some(revision) = snapshot.revision.0.checked_add(1) else {
                    snapshot.outcome = Outcome::Failed(Arc::new(DesktopError::Closed));
                    let _previous = acknowledged.send_replace(Acknowledged {
                        sequence,
                        revision: snapshot.revision,
                    });
                    let _previous = published.send_replace(snapshot);
                    break;
                };
                let next = merge::apply(&snapshot.prefs, &before, &after)
                    .map_err(DesktopError::from)
                    .and_then(|next| {
                        if let Some(path) = &path {
                            next.save_at(path).map_err(DesktopError::PreferencesWrite)?;
                        }
                        Ok(next)
                    });
                snapshot.revision = Revision(revision);
                snapshot.outcome = match next {
                    Ok(next) => {
                        snapshot.prefs = Arc::new(next);
                        if path.is_some() {
                            Outcome::Saved
                        } else {
                            Outcome::LaunchOnly
                        }
                    }
                    Err(error) => Outcome::Failed(Arc::new(error)),
                };
                let _previous = acknowledged.send_replace(Acknowledged {
                    sequence,
                    revision: snapshot.revision,
                });
                let _previous = published.send_replace(snapshot.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_app_view::prefs::{Grouping, Theme};

    #[tokio::test]
    async fn older_ack_keeps_pending_intent_and_old_failure_cannot_roll_back_newer_commit() {
        let initial = Prefs::default();
        let coordinator = Coordinator::start(initial.clone()).unwrap();
        let (client, _updates) = coordinator.client();
        let original_canonical = client.canonical_preferences();
        let mut first = initial;
        first.settings.appearance.theme = Theme::Light;
        client.publish(&first).unwrap();
        assert_eq!(*client.canonical_preferences(), *original_canonical);
        coordinator.flush().await.unwrap();
        let older = coordinator.snapshot();
        let mut next = first;
        next.settings.sidebar.group = Grouping::Status;
        client.publish(&next).unwrap();
        // This explicit older snapshot remains deterministic even if the writer
        // has already acknowledged the newer request on the other thread.
        let Observation::Applied(projected) = client.observe(&older) else {
            panic!("valid projection");
        };
        assert_eq!(projected.settings.sidebar.group, Grouping::Status);
        assert_eq!(*client.canonical_preferences(), *older.preferences());
        coordinator.flush().await.unwrap();
        let Observation::Applied(committed) = client.observe(&coordinator.snapshot()) else {
            panic!("current projection");
        };
        assert_eq!(committed.settings.sidebar.group, Grouping::Status);
        assert_eq!(*client.canonical_preferences(), *coordinator.current());
        let mut old_failure = older;
        old_failure.outcome = Outcome::Failed(Arc::new(DesktopError::Closed));
        assert!(matches!(client.observe(&old_failure), Observation::Stale));
        assert_eq!(
            client.baseline.borrow().settings.sidebar.group,
            Grouping::Status
        );
    }

    #[tokio::test]
    async fn stale_windows_merge_and_failed_save_keeps_canonical_state() {
        let root =
            std::env::temp_dir().join(format!("vessel-central-prefs-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let initial = Prefs {
            kept: Kept::File {
                path: root.join("prefs.json").display().to_string(),
            },
            ..Prefs::default()
        };
        let coordinator = Coordinator::start(initial.clone()).unwrap();
        let (first, _updates) = coordinator.client();
        let (second, mut updates) = coordinator.client();
        let mut one = initial.clone();
        one.settings.appearance.theme = Theme::Light;
        first.publish(&one).unwrap();
        coordinator.flush().await.unwrap();
        let mut two = initial;
        two.settings.sidebar.group = Grouping::Status;
        second.publish(&two).unwrap();
        coordinator.flush().await.unwrap();
        updates.changed().await.unwrap();
        assert_eq!(
            coordinator.current().settings.appearance.theme,
            Theme::Light
        );
        assert_eq!(
            coordinator.current().settings.sidebar.group,
            Grouping::Status
        );
        std::fs::remove_file(root.join("prefs.json")).unwrap();
        std::fs::create_dir(root.join("prefs.json")).unwrap();
        let committed = coordinator.current();
        let mut refused = (*committed).clone();
        refused.manual_updates = true;
        second.publish(&refused).unwrap();
        coordinator.flush().await.unwrap();
        assert_eq!(coordinator.current(), committed);
        assert!(matches!(updates.borrow().outcome(), Outcome::Failed(_)));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn corrupt_file_remains_launch_only_and_never_gets_replaced() {
        let root =
            std::env::temp_dir().join(format!("vessel-corrupt-prefs-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("prefs.json");
        std::fs::write(&path, b"unreadable").unwrap();
        let initial = Prefs {
            kept: Kept::ThisLaunch {
                why: vessel_app_view::prefs::LaunchOnly::Unreadable {
                    fault: vessel_app_view::files::FileFault {
                        path: path.display().to_string(),
                        at: None,
                        problem: vessel_app_view::files::FileProblem::CannotOpen,
                        field: None,
                    },
                },
            },
            ..Prefs::default()
        };
        let coordinator = Coordinator::start(initial.clone()).unwrap();
        let (client, updates) = coordinator.client();
        let mut next = initial;
        next.manual_updates = true;
        client.publish(&next).unwrap();
        coordinator.flush().await.unwrap();
        assert!(coordinator.current().manual_updates);
        assert!(matches!(updates.borrow().outcome(), Outcome::LaunchOnly));
        assert_eq!(std::fs::read(path).unwrap(), b"unreadable");
        std::fs::remove_dir_all(root).unwrap();
    }
}
