//! The slots git runs take: one pool for local runs and one for network
//! runs, each with a bound on the runs that wait for it. It runs nothing.
//!
//! [Decision Log]
//! - At most [`LOCAL_SLOTS`] local and [`NETWORK_SLOTS`] network runs at
//!   once per node; the next waits for a slot (GOAT and Paseo use the same
//!   bound).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::git::GitError;

/// Local git runs one node makes at once (status, diff, commit).
pub(crate) const LOCAL_SLOTS: usize = 8;
/// Network git runs one node makes at once (clone, fetch, push, pull), kept
/// apart so a slow clone never holds up a status.
pub(crate) const NETWORK_SLOTS: usize = 4;
/// Runs that may wait for a slot of one pool; past that a call answers
/// `unavailable` at once instead of queueing without end.
pub(crate) const MAX_WAITING: usize = 64;

/// One pool of slots: at most `slots` runs at once, at most
/// [`MAX_WAITING`] waiting.
#[derive(Debug)]
pub(super) struct Pool {
    slots: Arc<Semaphore>,
    waiting: AtomicUsize,
}

impl Pool {
    pub(super) fn new(slots: usize) -> Arc<Self> {
        Arc::new(Self {
            slots: Arc::new(Semaphore::new(slots)),
            waiting: AtomicUsize::new(0),
        })
    }

    /// A slot, or [`GitError::Busy`] when too many runs already wait.
    pub(super) async fn slot(&self) -> Result<OwnedSemaphorePermit, GitError> {
        if let Ok(permit) = Arc::clone(&self.slots).try_acquire_owned() {
            return Ok(permit);
        }
        let waiting = self.waiting.fetch_add(1, Ordering::AcqRel);
        let _counted = Counted(&self.waiting);
        if waiting >= MAX_WAITING {
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
