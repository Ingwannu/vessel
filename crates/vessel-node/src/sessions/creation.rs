//! Staged actor-registration join, below authorization/placement/intent admission.
//! It never resolves caller cwd, mints a second count entry, or sends first input.
use super::*;
use crate::agents::AgentReservation;
use vessel_proto::creation::{Advance, CreationError, Receipt, Resource, Stage};
use vessel_proto::registry::WorkspaceResource;
use vessel_proto::{SessionListing, registry::WorkspaceVisibility};
use vessel_store::{RegistryStartState, StoreError, WorkspaceStartLease};

impl Sessions {
    /// Validate before receipt/count admission; the registration join repeats it
    /// against the authoritative stored workspace. No caller cwd grants ownership.
    pub(crate) fn with_workspace_listing(
        mut spec: SessionSpec,
        visibility: WorkspaceVisibility,
    ) -> Result<SessionSpec, CreationError> {
        let listing = match visibility {
            WorkspaceVisibility::Foreground => SessionListing::Shown,
            WorkspaceVisibility::Background => SessionListing::Background,
        };
        if spec.listing.is_some_and(|requested| requested != listing) {
            return Err(CreationError::IntentConflict);
        }
        spec.listing = Some(listing);
        Ok(spec)
    }

    pub(crate) async fn create_receipted(
        self: &Arc<Self>,
        spec: SessionSpec,
        mut options: LaunchOptions,
        count: &mut AgentReservation<'_>,
        start: WorkspaceStartLease,
        begin: &Receipt,
    ) -> Result<Receipt, ServiceError> {
        // Bind count retention before any verification await can be cancelled.
        count.begin(begin).map_err(StoreError::from)?;
        let reserved = &begin.snapshot().reserved;
        let Some(Resource::Agent { session }) = reserved.resource else {
            return Err(StoreError::from(CreationError::InvalidTransition).into());
        };
        if begin.in_flight() != Some(Stage::Resource)
            || start.workspace() != reserved.workspace
            || start.resource() != (WorkspaceResource::Session { session })
            || options.id.is_some_and(|id| id != session)
            || self.runtime.get(session).is_some()
        {
            return Err(StoreError::from(CreationError::InvalidTransition).into());
        }
        let current = self.store.creation_operation(reserved.operation).await?;
        if current.as_ref() != Some(begin) {
            return Err(StoreError::from(CreationError::StaleRevision).into());
        }
        let pending = self
            .store
            .pending_registry_starts(reserved.workspace)
            .await?;
        if !pending
            .iter()
            .any(|entry| entry.lease == start && entry.state == RegistryStartState::Pending)
        {
            return Err(StoreError::from(CreationError::InvalidTransition).into());
        }
        let workspace = self
            .store
            .registry_workspace(reserved.workspace)
            .await?
            .ok_or(CreationError::InvalidTransition)
            .map_err(StoreError::from)?;
        let spec = Self::with_workspace_listing(spec, workspace.descriptor.visibility)
            .map_err(StoreError::from)?;
        match self.find(session).await {
            Err(ServiceError::NotFound(id)) if id == session => {}
            Ok(_) => return Err(StoreError::from(CreationError::ResourceClaimed).into()),
            Err(error) => return Err(error),
        }
        options.id = Some(session);
        // Existing tracked creation finishes actor registration and persistence
        // even if its caller disappears. After Begin, errors/cancellation retain
        // count/start/receipt ownership; none is proof of a no-spawn refusal.
        let _registered = self.create(spec, options, None).await?;
        self.store.complete_registry_start(start).await?;
        self.store
            .advance_creation(
                reserved.operation,
                begin.snapshot().revision,
                Advance::Finished(Stage::Resource),
            )
            .await
            .map_err(ServiceError::from)
    }
}
