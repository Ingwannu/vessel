//! Independent workspace registry calls; person-only policy precedes these handlers.
//! Creation and lifecycle orchestration use the same Store, never a second registry.

use super::{Connection, NodeService};
use crate::{principal::Principal, waits::Waiter, workspace::MAX_WAIT_SECS};
use std::time::Duration;
use vessel_proto::registry::{
    DiffStatRevision, GetRegistry, GetWorkspace, ListWorkspaces, PinWorkspace, RegistrySnapshot,
    RenameWorkspace, ServedWorkspace, WorkspaceDescriptor, WorkspaceLifecycle, WorkspaceList,
    WorkspaceVisibility, WorkspaceVisibilityFilter, serves_diff_stat,
};
use vessel_proto::{ErrorCode, WireError, WorkspaceId};
use vessel_store::{RegistryRecord, StoreError};

fn wire(error: &StoreError) -> WireError {
    WireError::new(error.code(), error.to_string())
}

impl NodeService {
    pub(crate) async fn release_workspace_automation(
        &self,
        _: &Connection,
        request: vessel_proto::registry::ReleaseWorkspaceAutomation,
    ) -> Result<WorkspaceDescriptor, WireError> {
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
        if matches!(connection.principal(), Principal::Delegated { .. }) {
            wait = None;
        }
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
        let snapshot = RegistrySnapshot {
            labels: stored.labels,
            node: here,
            revision: stored.revision,
            workspaces: stored.workspaces,
            memberships: stored.memberships,
        };
        Ok(snapshot
            .for_protocol(connection.protocol())
            .unwrap_or(snapshot))
    }

    pub(crate) async fn list_workspaces(
        &self,
        connection: &Connection,
        request: ListWorkspaces,
    ) -> Result<Vec<ServedWorkspace>, WireError> {
        let stored = self
            .sessions
            .store()
            .workspace_registry()
            .await
            .map_err(|error| wire(&error))?;
        let changed_from = request.changed_from;
        let workspaces = stored
            .workspaces
            .into_iter()
            .filter(|workspace| listed(workspace, &request))
            .collect();
        Ok(self
            .with_diff_stat(connection, changed_from, workspaces)
            .await)
    }

    pub(crate) async fn get_workspace(
        &self,
        connection: &Connection,
        request: GetWorkspace,
    ) -> Result<ServedWorkspace, WireError> {
        let changed_from = request.changed_from;
        let descriptor = self.registry_record(request.workspace).await?.descriptor;
        let served = self
            .with_diff_stat(connection, changed_from, vec![descriptor])
            .await;
        one_served(served)
    }

    /// Attach the served diff stat, waiting when the caller asked to.
    ///
    /// While the protocol is unassigned this is the descriptor alone: no
    /// watch, no wait, no extra fields.
    async fn with_diff_stat(
        &self,
        connection: &Connection,
        changed_from: Option<DiffStatRevision>,
        workspaces: Vec<WorkspaceDescriptor>,
    ) -> Vec<ServedWorkspace> {
        if !serves_diff_stat(connection.protocol()) {
            return workspaces.into_iter().map(ServedWorkspace::bare).collect();
        }
        let mut changed = self.diff_stats.subscribe();
        let pairs: Vec<_> = workspaces
            .iter()
            .map(|workspace| (workspace.path.clone(), workspace.root()))
            .collect();
        let keys = self.diff_stats.note(&pairs).await;
        if keys.len() != workspaces.len() {
            tracing::error!("diff stat note returned a different number of folders");
            return workspaces.into_iter().map(ServedWorkspace::bare).collect();
        }
        let generation = self.diff_stats.generation();
        let pending = keys.iter().any(|key| !self.diff_stats.ready(key));
        let same = changed_from.is_some_and(|revision| revision.get() == generation);
        let mut wait = if same || (changed_from.is_none() && pending) {
            Some(MAX_WAIT_SECS)
        } else {
            None
        };
        if matches!(connection.principal(), Principal::Delegated { .. }) {
            wait = None;
        }
        let here = self.links.node();
        let shares = Waiter::of(connection.principal(), here).shares(here);
        let _slot = self.wait_slot(connection, shares, &mut wait);
        if let Some(seconds) = wait {
            let _settled = tokio::time::timeout(
                Duration::from_secs(u64::from(seconds)),
                changed.changed(),
            )
            .await;
        } else {
            drop(changed);
        }
        let generation = DiffStatRevision::of(self.diff_stats.generation());
        workspaces
            .into_iter()
            .zip(keys)
            .map(|(workspace, key)| ServedWorkspace {
                diff_stat: self.diff_stats.stat(&key),
                stat_revision: Some(generation),
                workspace,
            })
            .collect()
    }

    pub(crate) async fn rename_workspace(
        &self,
        _: &Connection,
        request: RenameWorkspace,
    ) -> Result<WorkspaceDescriptor, WireError> {
        let mut record = self.active_registry_record(request.workspace).await?;
        record.descriptor.title = request.title;
        self.registry_update(record).await
    }

    pub(crate) async fn pin_workspace(
        &self,
        _: &Connection,
        request: PinWorkspace,
    ) -> Result<WorkspaceDescriptor, WireError> {
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

fn listed(workspace: &WorkspaceDescriptor, request: &ListWorkspaces) -> bool {
    let project = request
        .project
        .is_none_or(|project| project == workspace.project);
    let lifecycle = match request.lifecycle {
        WorkspaceList::All => true,
        WorkspaceList::Active => matches!(workspace.lifecycle, WorkspaceLifecycle::Active),
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
}

fn one_served(served: Vec<ServedWorkspace>) -> Result<ServedWorkspace, WireError> {
    let mut served = served.into_iter();
    let Some(one) = served.next() else {
        return Err(missing_stat());
    };
    if served.next().is_some() {
        return Err(missing_stat());
    }
    Ok(one)
}

fn missing_stat() -> WireError {
    WireError::new(
        ErrorCode::Internal,
        "the workspace diff stat was not attached",
    )
}
