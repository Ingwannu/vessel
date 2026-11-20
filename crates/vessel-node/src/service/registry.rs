//! Independent workspace registry calls; person-only policy precedes these handlers.
//! Creation and lifecycle orchestration use the same Store, never a second registry.

use super::{Connection, NodeService};
use crate::{
    principal::Principal,
    waits::Waiter,
    workspace::{MAX_WAIT_SECS, should_wait},
};
use std::path::{Path, PathBuf};
use std::time::Duration;
use vessel_proto::registry::{
    DiffStat, DiffStatRevision, GetRegistry, GetWorkspace, ListWorkspaces, PinWorkspace,
    RegistrySnapshot, RenameWorkspace, ServedWorkspace, WorkspaceDescriptor, WorkspaceLifecycle,
    WorkspaceList, WorkspaceVisibility, WorkspaceVisibilityFilter, serves_diff_stat,
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
        let peer = stat_peer(connection);
        Ok(self
            .with_diff_stat(
                peer.served,
                peer.delegated,
                connection,
                changed_from,
                workspaces,
            )
            .await)
    }

    pub(crate) async fn get_workspace(
        &self,
        connection: &Connection,
        request: GetWorkspace,
    ) -> Result<ServedWorkspace, WireError> {
        let changed_from = request.changed_from;
        let descriptor = self.registry_record(request.workspace).await?.descriptor;
        let peer = stat_peer(connection);
        let rows = self
            .with_diff_stat(
                peer.served,
                peer.delegated,
                connection,
                changed_from,
                vec![descriptor],
            )
            .await;
        one_served(rows)
    }

    /// Attach the served diff stat when `served`, waiting when the caller
    /// asked to.
    ///
    /// `served` is the peer's [`serves_diff_stat`]. While it is false this
    /// is the descriptor alone: no count, no wait, no extra fields. A
    /// delegated caller is noted and does not wait. Tests pass `served`
    /// through [`should_wait`] and [`attached`] without assigning the
    /// protocol constant.
    async fn with_diff_stat(
        &self,
        served: bool,
        delegated: bool,
        connection: &Connection,
        changed_from: Option<DiffStatRevision>,
        workspaces: Vec<WorkspaceDescriptor>,
    ) -> Vec<ServedWorkspace> {
        if !served {
            return workspaces.into_iter().map(ServedWorkspace::bare).collect();
        }
        // `subscribe` marks the current generation seen, so the wait is for a
        // later one. A bump inside `note` stays unseen and wakes it.
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
        let mut wait = should_wait(
            changed_from.map(DiffStatRevision::get),
            generation,
            pending,
            delegated,
        )
        .then_some(MAX_WAIT_SECS);
        let here = self.links.node();
        let shares = Waiter::of(connection.principal(), here).shares(here);
        let _slot = self.wait_slot(connection, shares, &mut wait);
        if let Some(seconds) = wait {
            let _settled =
                tokio::time::timeout(Duration::from_secs(u64::from(seconds)), changed.changed())
                    .await;
        } else {
            drop(changed);
        }
        let generation = DiffStatRevision::of(self.diff_stats.generation());
        attached(workspaces, &keys, generation, |key| {
            self.diff_stats.stat(key)
        })
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

struct StatPeer {
    served: bool,
    delegated: bool,
}

/// Whether this peer is served a diff stat, and whether the call is delegated.
fn stat_peer(connection: &Connection) -> StatPeer {
    StatPeer {
        served: serves_diff_stat(connection.protocol()),
        delegated: matches!(connection.principal(), Principal::Delegated { .. }),
    }
}

/// One shared revision, and each folder's own count.
fn attached(
    workspaces: Vec<WorkspaceDescriptor>,
    keys: &[PathBuf],
    generation: DiffStatRevision,
    stat: impl Fn(&Path) -> Option<DiffStat>,
) -> Vec<ServedWorkspace> {
    workspaces
        .into_iter()
        .zip(keys.iter())
        .map(|(workspace, key)| ServedWorkspace {
            diff_stat: stat(key.as_path()),
            stat_revision: Some(generation),
            workspace,
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::DateTime;
    use vessel_proto::registry::{
        DiffStat, DiffStatRevision, RegistryRevision, ServedWorkspace, WorkspaceAutomation,
        WorkspaceBacking, WorkspaceDescriptor, WorkspaceLifecycle, WorkspacePin,
        WorkspaceVisibility,
    };
    use vessel_proto::{ProjectId, WorkspaceId};

    use super::attached;

    fn descriptor(path: &str) -> WorkspaceDescriptor {
        WorkspaceDescriptor {
            id: WorkspaceId::random(),
            project: ProjectId::random(),
            path: PathBuf::from(path),
            automation: WorkspaceAutomation::Allowed,
            title: None,
            pin: WorkspacePin::Unpinned,
            backing: WorkspaceBacking::Local,
            lifecycle: WorkspaceLifecycle::Active,
            visibility: WorkspaceVisibility::Foreground,
            created_at: DateTime::UNIX_EPOCH,
            revision: RegistryRevision::ZERO,
        }
    }

    #[test]
    fn rows_share_one_revision_and_keep_each_count() {
        let left = descriptor("/work/left");
        let right = descriptor("/work/right");
        let keys = vec![left.path.clone(), right.path.clone()];
        let generation = DiffStatRevision::of(4);
        let rows = attached(vec![left, right], &keys, generation, |key| {
            if key.ends_with("left") {
                Some(DiffStat {
                    additions: 1,
                    deletions: 2,
                })
            } else {
                None
            }
        });
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].stat_revision, Some(generation));
        assert_eq!(rows[1].stat_revision, Some(generation));
        assert_eq!(
            rows[0].diff_stat,
            Some(DiffStat {
                additions: 1,
                deletions: 2
            })
        );
        assert!(rows[1].diff_stat.is_none());
        let bare = ServedWorkspace::bare(descriptor("/work/one"));
        assert!(bare.diff_stat.is_none());
        assert!(bare.stat_revision.is_none());
    }
}
