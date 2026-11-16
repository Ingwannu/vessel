//! Fixed-size Saved delivery shares the canonical writer's publication boundary.

use super::{DesktopError, Revision, Snapshot};
use std::sync::{
    Arc, Mutex, MutexGuard, Weak,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::watch;
use vessel_app_view::terminal::ScrollbackLines;

/// The last successful terminal setting, distinct from an optimistic preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommittedScrollback {
    limit: ScrollbackLines,
    revision: Revision,
}

impl CommittedScrollback {
    pub(super) const fn new(limit: ScrollbackLines, revision: Revision) -> Self {
        Self { limit, revision }
    }
    #[must_use]
    /// The validated limit confirmed by this record.
    pub const fn limit(self) -> ScrollbackLines {
        self.limit
    }
    #[must_use]
    /// The existing canonical revision that confirmed this terminal value.
    pub const fn revision(self) -> Revision {
        self.revision
    }
}

/// Coalescing must preserve a successful intermediate shrink before an increase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbackBatch {
    minimum: ScrollbackLines,
    latest: CommittedScrollback,
}

impl ScrollbackBatch {
    fn new(latest: CommittedScrollback) -> Self {
        Self {
            minimum: latest.limit,
            latest,
        }
    }
    fn append(&mut self, latest: CommittedScrollback) {
        if latest.limit.get() < self.minimum.get() {
            self.minimum = latest.limit;
        }
        self.latest = latest;
    }
    #[must_use]
    /// Apply this minimum to the current owner before the final limit.
    pub const fn minimum(self) -> ScrollbackLines {
        self.minimum
    }
    #[must_use]
    /// The last committed value in the coalesced delivery.
    pub const fn latest(self) -> CommittedScrollback {
        self.latest
    }
}

#[derive(Debug)]
struct Slot {
    identity: Weak<()>,
    latest: Option<CommittedScrollback>,
    pending: Option<ScrollbackBatch>,
}

impl Slot {
    fn new(identity: Weak<()>, latest: Option<CommittedScrollback>) -> Self {
        Self {
            identity,
            latest,
            pending: latest.map(ScrollbackBatch::new),
        }
    }
    fn offer(&mut self, latest: Option<CommittedScrollback>) {
        if latest == self.latest {
            return;
        }
        if let Some(latest) = latest {
            match &mut self.pending {
                Some(pending) => pending.append(latest),
                None => self.pending = Some(ScrollbackBatch::new(latest)),
            }
            self.latest = Some(latest);
        }
    }
}

#[derive(Debug)]
struct Inner {
    slots: Mutex<Vec<Slot>>,
    healthy: AtomicBool,
}

/// This is effect delivery per existing Client, never a window registry or store.
#[derive(Debug, Clone)]
pub(super) struct Hub(Arc<Inner>);

impl Default for Hub {
    fn default() -> Self {
        Self(Arc::new(Inner {
            slots: Mutex::new(Vec::new()),
            healthy: AtomicBool::new(true),
        }))
    }
}

impl Hub {
    pub(super) fn healthy(&self) -> bool {
        self.0.healthy.load(Ordering::Acquire) && !self.0.slots.is_poisoned()
    }

    fn state(&self) -> Result<MutexGuard<'_, Vec<Slot>>, DesktopError> {
        if !self.healthy() {
            return Err(DesktopError::Closed);
        }
        match self.0.slots.lock() {
            Ok(state) if self.healthy() => Ok(state),
            Ok(_state) => Err(DesktopError::Closed),
            Err(error) => {
                self.0.healthy.store(false, Ordering::Release);
                tracing::warn!(%error, "committed preference delivery became unavailable");
                Err(DesktopError::Closed)
            }
        }
    }

    /// Clone the watch value under the same lock as publication. Never retain a
    /// watch borrow while entering a hub operation; callers use owned snapshots.
    pub(super) fn subscribe(
        &self,
        current: &watch::Receiver<Snapshot>,
    ) -> (Snapshot, Subscription) {
        let identity = Arc::new(());
        let snapshot = match self.state() {
            Ok(mut slots) => {
                let snapshot = current.borrow().clone();
                slots.retain(|slot| slot.identity.strong_count() > 0);
                slots.push(Slot::new(Arc::downgrade(&identity), snapshot.scrollback));
                snapshot
            }
            Err(error) => {
                tracing::warn!(%error, "new preference client has no committed delivery");
                current.borrow().clone()
            }
        };
        (
            snapshot,
            Subscription {
                hub: self.clone(),
                identity,
            },
        )
    }

    /// No I/O or UI callback runs under this lock. A drain cannot see a partial
    /// slot update before the canonical watch publication completes.
    pub(super) fn publish(
        &self,
        snapshot: Snapshot,
        published: &watch::Sender<Snapshot>,
    ) -> Result<(), DesktopError> {
        let mut slots = self.state()?;
        slots.retain(|slot| slot.identity.strong_count() > 0);
        for slot in slots.iter_mut() {
            slot.offer(snapshot.scrollback);
        }
        let _previous = published.send_replace(snapshot);
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct Subscription {
    hub: Hub,
    identity: Arc<()>,
}

#[derive(Debug)]
pub(super) struct Drain {
    pub(super) batch: Option<ScrollbackBatch>,
    pub(super) latest: Option<CommittedScrollback>,
}

impl Subscription {
    pub(super) fn healthy(&self) -> bool {
        self.hub.healthy()
    }

    pub(super) fn drain(&self) -> Result<Drain, DesktopError> {
        let mut slots = self.hub.state()?;
        let identity = Arc::downgrade(&self.identity);
        let slot = slots
            .iter_mut()
            .find(|slot| slot.identity.ptr_eq(&identity))
            .ok_or(DesktopError::Closed)?;
        Ok(Drain {
            batch: slot.pending.take(),
            latest: slot.latest,
        })
    }

    pub(super) fn settled(&self, applied: Option<CommittedScrollback>) -> bool {
        self.hub.state().is_ok_and(|slots| {
            let identity = Arc::downgrade(&self.identity);
            slots
                .iter()
                .find(|slot| slot.identity.ptr_eq(&identity))
                .is_some_and(|slot| slot.pending.is_none() && slot.latest == applied)
        })
    }
}

#[cfg(test)]
mod tests;
