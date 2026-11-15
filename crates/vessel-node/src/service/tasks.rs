//! Preserve the node's distinct work and subscription retirement obligations.
//! Services borrow these existing trackers; this bundle creates no new tracker.

use tokio_util::task::TaskTracker;

/// The node owns both trackers; services distinguish real work from reply grace.
pub(crate) struct ServiceTasks {
    pub(super) work: TaskTracker,
    pub(super) forwarders: TaskTracker,
}
impl ServiceTasks {
    /// Background work must finish before the store closes; subscriptions get grace.
    pub(crate) fn new(work: &TaskTracker, forwarders: &TaskTracker) -> Self {
        Self {
            work: work.clone(),
            forwarders: forwarders.clone(),
        }
    }
}
