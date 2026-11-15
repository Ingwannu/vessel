//! Discovery policies over the canonical collection, and creation inheritance.
//! Exact-id access never consults this module's inclusion predicate.

use vessel_proto::{SessionId, SessionInfo, SessionListing, SessionSelection, SessionSpec};

use super::Sessions;
use crate::error::ServiceError;

impl Sessions {
    /// Filter a fresh read, never the canonical collection or its ordering.
    pub(crate) async fn selected(
        &self,
        selection: &SessionSelection,
    ) -> Result<Vec<SessionInfo>, ServiceError> {
        let mut sessions = self.list().await?;
        sessions.retain(|info| {
            selection.scope.includes(info.listing) && info.labels.covers(&selection.labels)
        });
        Ok(sessions)
    }

    /// Resolve only omitted Background inheritance. Shown stays the legacy
    /// omission, so ordinary creation still works through older peers.
    pub(crate) async fn inherit_listing(
        &self,
        spec: &mut SessionSpec,
        local_creator: Option<SessionId>,
    ) -> Result<(), ServiceError> {
        if spec.listing.is_none()
            && let Some(parent) = local_creator
            && self.get(parent).await?.listing == SessionListing::Background
        {
            spec.listing = Some(SessionListing::Background);
        }
        Ok(())
    }
}
