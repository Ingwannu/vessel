//! The slots git runs take: one pool for local runs and one for network
//! runs, each with a bound on the runs that wait for it. It runs nothing.
//!
//! [Decision Log]
//! - At most [`LOCAL_SLOTS`] local, [`NETWORK_SLOTS`] network and
//!   [`CLONE_SLOTS`] clone runs at once per node; the next waits for a
//!   slot (GOAT and Paseo use the same bound for the first two).
//! - The wait counts against the call's time limit and ends with its
//!   cancellation (`GitCommand::output_streaming`): a call behind slow runs
//!   answers `timed out` within its time instead of queueing until a slot
//!   frees.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::git::GitError;

/// Local git runs one node makes at once (status, diff, commit).
pub(crate) const LOCAL_SLOTS: usize = 8;
/// Network git runs one node makes at once (fetch, push, pull), kept
/// apart so a slow remote never holds up a status.
pub(crate) const NETWORK_SLOTS: usize = 4;
/// Clones one node makes at once (ADR 0030, wave 1's review finding 11):
/// a pool of their own, since a clone may take an hour and four of them
/// would otherwise hold every network slot a person's push waits for.
pub(crate) const CLONE_SLOTS: usize = 2;
/// Runs that may wait for a slot of the local or network pool; past that
/// a call answers `unavailable` at once instead of queueing without end.
pub(crate) const MAX_WAITING: usize = 64;
/// Clones that may wait for a slot.
pub(crate) const CLONES_WAITING: usize = 16;

/// One pool of slots: at most `slots` runs at once, at most `max_waiting`
/// waiting.
#[derive(Debug)]
pub(super) struct Pool {
    slots: Arc<Semaphore>,
    waiting: AtomicUsize,
    max_waiting: usize,
}

impl Pool {
    pub(super) fn new(slots: usize, max_waiting: usize) -> Arc<Self> {
        Arc::new(Self {
            slots: Arc::new(Semaphore::new(slots)),
            waiting: AtomicUsize::new(0),
            max_waiting,
        })
    }

    /// A slot, or [`GitError::Busy`] when too many runs already wait.
    pub(super) async fn slot(&self) -> Result<OwnedSemaphorePermit, GitError> {
        if let Ok(permit) = Arc::clone(&self.slots).try_acquire_owned() {
            return Ok(permit);
        }
        let waiting = self.waiting.fetch_add(1, Ordering::AcqRel);
        let _counted = Counted(&self.waiting);
        if waiting >= self.max_waiting {
            return Err(GitError::Busy);
        }
        Arc::clone(&self.slots)
            .acquire_owned()
            .await
            .map_err(|_closed| GitError::Closed)
    }
}

/// Takes one off a waiting count when dropped (also when the waiting call
/// is dropped).
struct Counted<'a>(&'a AtomicUsize);

impl Drop for Counted<'_> {
    fn drop(&mut self) {
        let _previous = self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
