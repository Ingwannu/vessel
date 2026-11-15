//! [`TerminalHandle`]: what the node holds of a terminal, cheap to clone.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use vessel_proc::ProcessLifetime;
use vessel_proto::terminals::TerminalInfo;
use vessel_proto::{EventRecord, EventSeq, SessionTitle, TerminalId, TerminalInput};
use vessel_pty::{PtyInput, WindowSize};

use crate::ring::{Follow, FollowError, Ring};

/// What a terminal's actor shares with its handles.
#[derive(Debug)]
pub(crate) struct Shared {
    /// The actor's actual completion; status and ProcessLifetime cannot mint it.
    pub(super) cleanup: super::cleanup::Progress,
    pub(crate) info: watch::Sender<TerminalInfo>,
    pub(crate) ring: Mutex<Ring>,
    /// A shell's input; scripts take none.
    pub(crate) input: Option<PtyInput>,
    /// Ends it (a child of the node's shutdown token).
    pub(crate) kill: CancellationToken,
    /// A script's actor takes restarts here.
    pub(crate) restarts: Option<mpsc::Sender<()>>,
    /// The running process's id, 0 when none: the root of its tree for a
    /// port scan.
    pub(crate) pid: AtomicU32,
    /// The real runner's observation; no handle owns revocation authority.
    pub(crate) process: Mutex<Option<ProcessLifetime>>,
}

impl Shared {
    pub(crate) fn ring(&self) -> MutexGuard<'_, Ring> {
        self.ring.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A page of a terminal's log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Records from the one asked for (or the oldest kept one), in order.
    pub records: Vec<Arc<EventRecord>>,
    /// The number to ask for next.
    pub next: EventSeq,
    /// Whether `next` is the end of the log now.
    pub at_end: bool,
}

/// Why input did not reach a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InputError {
    /// A script or a service: it takes no input.
    #[error("a script's run takes no input")]
    NoInput,
    /// It ended.
    #[error("the terminal has ended")]
    Ended,
}

/// Why a run could not be restarted in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RestartError {
    /// A shell (only runs restart).
    #[error("only a script's run can be restarted")]
    NotARun,
    /// It ended: run it again in a new terminal.
    #[error("the run has ended")]
    Ended,
}

/// A terminal, live or ended. Clones share it.
#[derive(Debug, Clone)]
pub struct TerminalHandle(pub(crate) Arc<Shared>);

impl TerminalHandle {
    /// Its id.
    #[must_use]
    pub fn id(&self) -> TerminalId {
        self.0.info.borrow().id
    }

    /// It as it is now.
    #[must_use]
    pub fn info(&self) -> TerminalInfo {
        self.0.info.borrow().clone()
    }

    /// Its info's changes (every change of `revision` wakes the receiver;
    /// output does not).
    #[must_use]
    pub fn watch(&self) -> watch::Receiver<TerminalInfo> {
        self.0.info.subscribe()
    }

    /// Whether its process ended for good.
    #[must_use]
    pub fn is_ended(&self) -> bool {
        !self.0.info.borrow().is_live()
    }

    /// Resolves once it ended for good.
    pub async fn ended(&self) {
        let mut info = self.watch();
        // Fails only if the sender is gone, which `self` holds.
        let _ended = info.wait_for(|info: &TerminalInfo| !info.is_live()).await;
    }

    /// Observe the actual runner after final cleanup, with unproved facts explicit.
    /// Waiting may outlive logical status; callers choose their own total deadline.
    pub async fn cleanup(&self) -> super::CleanupObservation {
        self.0.cleanup.observe().await
    }

    /// The running process's id (the root of its tree), if one runs.
    #[must_use]
    pub fn pid(&self) -> Option<u32> {
        match self.0.pid.load(Ordering::Relaxed) {
            0 => None,
            pid => Some(pid),
        }
    }

    /// Observe the current run rather than inferring ownership from a PID snapshot.
    #[must_use]
    pub fn process_lifetime(&self) -> Option<ProcessLifetime> {
        self.0
            .process
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .filter(|process| process.is_live())
            .cloned()
    }

    /// Kept records from `from` (from the oldest kept one when `from` is
    /// older: the scrollback is bounded), then the live stream.
    ///
    /// # Errors
    /// [`FollowError::Ahead`] when `from` is past the end of its log.
    pub fn follow(&self, from: EventSeq) -> Result<Follow, FollowError> {
        let ring = self.0.ring();
        match ring.follow(from) {
            Err(FollowError::Gap { oldest_cached }) => ring.follow(oldest_cached),
            other => other,
        }
    }

    /// A page from `from` (from the oldest kept one when older): at most
    /// `limit` records and about `max_bytes` of output (always one record
    /// when there is one).
    ///
    /// # Errors
    /// [`FollowError::Ahead`] when `from` is past the end of its log.
    pub fn page(
        &self,
        from: EventSeq,
        limit: usize,
        max_bytes: usize,
    ) -> Result<Page, FollowError> {
        let ring = self.0.ring();
        let next = ring.next();
        if from > next {
            return Err(FollowError::Ahead { next });
        }
        let records = ring.page(from, limit, max_bytes);
        let after = records
            .last()
            .and_then(|record| record.seq.next())
            .unwrap_or_else(|| from.max(ring.oldest()));
        Ok(Page {
            at_end: after == next,
            next: after,
            records,
        })
    }

    /// Write to a shell, or resize it (the last resize wins).
    ///
    /// # Errors
    /// [`InputError`].
    pub async fn send(&self, input: TerminalInput) -> Result<(), InputError> {
        let pty = self.0.input.as_ref().ok_or(InputError::NoInput)?;
        if self.is_ended() {
            return Err(InputError::Ended);
        }
        match input {
            TerminalInput::Data { data } => pty
                .write(data.into_bytes())
                .await
                .map_err(|_closed| InputError::Ended),
            TerminalInput::Resize { size } => {
                pty.resize(WindowSize {
                    cols: size.cols,
                    rows: size.rows,
                })
                .await
                .map_err(|_closed| InputError::Ended)?;
                self.0.info.send_modify(|info| {
                    info.size = Some(size);
                    info.revision = info.revision.wrapping_add(1);
                });
                Ok(())
            }
        }
    }

    /// Give it a title.
    pub fn rename(&self, title: SessionTitle) {
        self.0.info.send_modify(|info| {
            info.title = Some(title);
            info.revision = info.revision.wrapping_add(1);
        });
    }

    /// End its process: politely, then its group at the grace (a service
    /// is not started again). Nothing once it ended.
    pub fn kill(&self) {
        self.0.kill.cancel();
    }

    /// Stop a live run and run it again in place.
    ///
    /// # Errors
    /// [`RestartError`].
    pub async fn restart(&self) -> Result<(), RestartError> {
        let restarts = self.0.restarts.as_ref().ok_or(RestartError::NotARun)?;
        if self.is_ended() {
            return Err(RestartError::Ended);
        }
        restarts
            .send(())
            .await
            .map_err(|_ended| RestartError::Ended)
    }
}
