//! Independent workspace registry calls; person-only policy precedes these handlers.
//! Creation and lifecycle orchestration use the same Store, never a second registry.

use super::{Connection, NodeService};
use crate::{principal::Principal, waits::Waiter, workspace::MAX_WAIT_SECS};
use std::time::Duration;
use vessel_proto::registry::{
    GetRegistry, GetWorkspace, ListWorkspaces, PinWorkspace, RegistrySnapshot, RenameWorkspace,
    WorkspaceDescriptor, WorkspaceLifecycle, WorkspaceList, WorkspaceVisibility,
    WorkspaceVisibilityFilter,
};
use vessel_proto::{ErrorCode, WireError, WorkspaceId};
use vessel_store::{RegistryRecord, StoreError};

fn wire(error: &StoreError) -> WireError {
    WireError::new(error.code(), error.to_string())
}

// Registry metadata is host-wide, unlike a delegated session's own resources.
// The declared scope remains checked by Guarded; this local boundary additionally
// requires a direct person before any Store read, watcher or mutation.
fn require_person(principal: &Principal) -> Result<(), WireError> {
    match principal {
        Principal::Owner | Principal::Device { .. } => Ok(()),
        Principal::Agent { .. } | Principal::Delegated { .. } => Err(WireError::new(
            ErrorCode::Forbidden,
            "workspace metadata requires a direct person connection",
        )),
    }
}

impl NodeService {
    pub(crate) async fn release_workspace_automation(
        &self,
        connection: &Connection,
        request: vessel_proto::registry::ReleaseWorkspaceAutomation,
    ) -> Result<WorkspaceDescriptor, WireError> {
        require_person(connection.principal())?;
        let id = request.workspace;
        self.sessions
            .store()
            .release_registry_automation(request)
            .await
            .map_err(|error| wire(&error))?;
        self.registry_record(id)
            .await
            .map(|record| record.descriptor)
    }

    pub(crate) async fn get_registry(
        &self,
        connection: &Connection,
        request: GetRegistry,
    ) -> Result<RegistrySnapshot, WireError> {
        require_person(connection.principal())?;
        let store = self.sessions.store();
        let mut changes = store.registry_changes();
        let mut stored = store
            .workspace_registry()
            .await
            .map_err(|error| wire(&error))?;
        let mut wait = request
            .changed_from
            .filter(|revision| *revision == stored.revision)
            .map(|_| MAX_WAIT_SECS);
        let here = self.links.node();
        let shares = Waiter::of(connection.principal(), here).shares(here);
        let _slot = self.wait_slot(connection, shares, &mut wait);
        if let Some(seconds) = wait {
            let _settled =
                tokio::time::timeout(Duration::from_secs(u64::from(seconds)), changes.changed())
                    .await;
            stored = store
                .workspace_registry()
                .await
                .map_err(|error| wire(&error))?;
        }
        Ok(RegistrySnapshot {
            node: here,
            revision: stored.revision,
            workspaces: stored.workspaces,
            memberships: stored.memberships,
        })
    }

    pub(crate) async fn list_workspaces(
        &self,
        connection: &Connection,
        request: ListWorkspaces,
    ) -> Result<Vec<WorkspaceDescriptor>, WireError> {
        require_person(connection.principal())?;
        let stored = self
            .sessions
            .store()
            .workspace_registry()
            .await
            .map_err(|error| wire(&error))?;
        Ok(stored
            .workspaces
            .into_iter()
            .filter(|workspace| {
                let project = request
                    .project
                    .is_none_or(|project| project == workspace.project);
                let lifecycle = match request.lifecycle {
                    WorkspaceList::All => true,
                    WorkspaceList::Active => {
                        matches!(workspace.lifecycle, WorkspaceLifecycle::Active)
                    }
                    WorkspaceList::Archived => {
                        matches!(workspace.lifecycle, WorkspaceLifecycle::Archived { .. })
                    }
                };
                let visibility = match request.visibility {
                    WorkspaceVisibilityFilter::All => true,
                    WorkspaceVisibilityFilter::Foreground => {
                        workspace.visibility == WorkspaceVisibility::Foreground
                    }
                };
                project && lifecycle && visibility
            })
            .collect())
    }

    pub(crate) async fn get_workspace(
        &self,
        connection: &Connection,
        request: GetWorkspace,
    ) -> Result<WorkspaceDescriptor, WireError> {
        require_person(connection.principal())?;
        self.registry_record(request.workspace)
            .await
            .map(|record| record.descriptor)
    }

    pub(crate) async fn rename_workspace(
        &self,
        connection: &Connection,
        request: RenameWorkspace,
    ) -> Result<WorkspaceDescriptor, WireError> {
        require_person(connection.principal())?;
        let mut record = self.active_registry_record(request.workspace).await?;
        record.descriptor.title = request.title;
        self.registry_update(record).await
    }

    pub(crate) async fn pin_workspace(
        &self,
        connection: &Connection,
        request: PinWorkspace,
    ) -> Result<WorkspaceDescriptor, WireError> {
        require_person(connection.principal())?;
        let mut record = self.active_registry_record(request.workspace).await?;
        record.descriptor.pin = request.pin;
        self.registry_update(record).await
    }

    async fn registry_record(&self, id: WorkspaceId) -> Result<RegistryRecord, WireError> {
        self.sessions
            .store()
            .registry_workspace(id)
            .await
            .map_err(|error| wire(&error))?
            .ok_or_else(|| {
                WireError::new(
                    ErrorCode::NotFound,
                    "workspace is not registered on this node",
                )
            })
    }

    async fn active_registry_record(&self, id: WorkspaceId) -> Result<RegistryRecord, WireError> {
        let record = self.registry_record(id).await?;
        match record.descriptor.lifecycle {
            WorkspaceLifecycle::Active => Ok(record),
            WorkspaceLifecycle::Archiving { .. }
            | WorkspaceLifecycle::Restoring { .. }
            | WorkspaceLifecycle::Archived { .. } => Err(WireError::new(
                ErrorCode::Conflict,
                "workspace is not active",
            )),
        }
    }

    async fn registry_update(
        &self,
        mut record: RegistryRecord,
    ) -> Result<WorkspaceDescriptor, WireError> {
        let expected = record.descriptor.revision;
        record.descriptor.revision = expected.next().ok_or_else(|| {
            WireError::new(ErrorCode::Unavailable, "workspace revision is exhausted")
        })?;
        let descriptor = record.descriptor.clone();
        self.sessions
            .store()
            .put_registry_workspace(record, Some(expected))
            .await
            .map_err(|error| wire(&error))?;
        Ok(descriptor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{DeviceId, NodeId, Origin, OriginBy, Scope, SessionId};

    fn delegated(by: OriginBy) -> Principal {
        Principal::delegated(
            DeviceId::random(),
            &Scope::ALL,
            Origin {
                node: NodeId::random(),
                by,
                scopes: Scope::ALL.to_vec(),
            },
            NodeId::random(),
        )
    }

    #[test]
    fn registry_metadata_accepts_direct_people_without_replacing_scope_checks() {
        for principal in [
            Principal::Owner,
            Principal::Device {
                id: DeviceId::random(),
                scopes: Scope::ALL.to_vec(),
            },
        ] {
            assert!(require_person(&principal).is_ok());
        }
    }

    #[test]
    fn registry_metadata_refuses_every_delegated_origin_even_with_all_scopes() {
        for principal in [
            Principal::Agent {
                node: NodeId::random(),
                session: SessionId::random(),
                scopes: Scope::ALL.to_vec(),
            },
            delegated(OriginBy::Owner),
            delegated(OriginBy::Device {
                id: DeviceId::random(),
            }),
            delegated(OriginBy::Agent {
                session: SessionId::random(),
                depth: 0,
            }),
        ] {
            assert_eq!(
                require_person(&principal).unwrap_err().code,
                ErrorCode::Forbidden
            );
        }
    }
}
