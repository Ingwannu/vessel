//! Lend the existing hard-drain token without adding a task owner or quota.

use super::Sessions;

impl Sessions {
    /// Title work uses the node's hard drain, not the forwarders' grace.
    pub(crate) fn metadata_work(&self) -> tokio_util::task::task_tracker::TaskTrackerToken {
        self.tasks.token()
    }
}
