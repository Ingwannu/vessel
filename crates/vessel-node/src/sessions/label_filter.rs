//! Staged label discovery: the existing merged list remains the authority.
//!
//! `cfg(test)` is removed by the call-wiring commit after the shared
//! required-protocol accessor and dispatch plumbing reach the node head.
//! No caller can expose an unassigned filter before that admission exists.

use vessel_proto::{SessionInfo, SessionLabels};

use super::Sessions;
use crate::error::ServiceError;

impl Sessions {
    /// Filter the merged current metadata without changing visibility or order.
    pub(super) async fn matching_labels(
        &self,
        required: &SessionLabels,
    ) -> Result<Vec<SessionInfo>, ServiceError> {
        let mut sessions = self.list().await?;
        sessions.retain(|info| info.labels.covers(required));
        Ok(sessions)
    }
}
