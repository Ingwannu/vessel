//! Staged placement inputs retain canonical metadata, not physical-root authority.
use super::Sessions;
use crate::workspace::{ProjectAdmission, Workspaces};
use std::sync::Arc;
use tokio::sync::OwnedMutexGuard;
use vessel_proto::registry::{
    RegistryRevision, WorkspaceBacking, WorkspaceDescriptor, WorkspaceLifecycle, WorkspaceResource,
};
use vessel_proto::workspace::WorkspaceRoot;
use vessel_proto::{SessionId, WorkspaceId};
use vessel_store::{StoreError, WorkspaceStartLease};
use vessel_workspace::paths::Root;

#[derive(Debug, thiserror::Error)]
pub(crate) enum PlacementFailure {
    #[error("creation placement storage failed")]
    Store(#[from] StoreError),
    #[error("creation project admission failed")]
    Workspace(#[from] crate::workspace::WorkspaceError),
}

#[derive(Debug)]
pub(crate) struct BoundCreationPlace {
    pub(crate) parent: SessionId,
    pub(crate) child: SessionId,
    pub(crate) workspace: WorkspaceId,
    pub(crate) expected: RegistryRevision,
}

#[derive(Debug)]
pub(crate) struct CreationPlacement {
    parent: SessionId,
    child: SessionId,
    descriptor: WorkspaceDescriptor,
    root: Root,
    start: WorkspaceStartLease,
    // This existing global64 hold extends unrelated-project latency through
    // actor settlement. It is not a per-project or physical generation lease.
    _project: Arc<ProjectAdmission>,
    _owner: Arc<Sessions>,
    _metadata: OwnedMutexGuard<()>,
}

impl Sessions {
    /// Project admission MUST be acquired before this existing metadata lock,
    /// matching removal's project→metadata order. Only a tracked owner may call:
    /// interrupted pending starts remain retained, never implicitly failed.
    pub(crate) async fn hold_creation_placement(
        self: &Arc<Self>,
        workspaces: &Arc<Workspaces>,
        project: Arc<ProjectAdmission>,
        request: BoundCreationPlace,
    ) -> Result<CreationPlacement, PlacementFailure> {
        let metadata = Arc::clone(&self.marking).lock_owned().await;
        let snapshot = self.store.workspace_registry().await?;
        if !snapshot.memberships.iter().any(|membership| {
            membership.workspace == request.workspace
                && membership.resource
                    == WorkspaceResource::Session {
                        session: request.parent,
                    }
        }) {
            return Err(StoreError::RegistryConflict.into());
        }
        let descriptor = snapshot
            .workspaces
            .into_iter()
            .find(|descriptor| descriptor.id == request.workspace)
            .ok_or(StoreError::RegistryConflict)?;
        if descriptor.lifecycle != WorkspaceLifecycle::Active
            || descriptor.revision != request.expected
        {
            return Err(StoreError::RegistryConflict.into());
        }
        project.require(workspaces, descriptor.project)?;
        let key = match descriptor.backing {
            WorkspaceBacking::Local => WorkspaceRoot::Project {
                project: descriptor.project,
            },
            WorkspaceBacking::Managed { worktree } => WorkspaceRoot::Worktree { worktree },
        };
        let resolved = workspaces.resolve(key).await?;
        if resolved.project != descriptor.project
            || resolved.root.path() != descriptor.path.as_path()
        {
            return Err(StoreError::RegistryConflict.into());
        }
        let start = self
            .store
            .claim_registry_start(
                request.workspace,
                WorkspaceResource::Session {
                    session: request.child,
                },
                request.expected,
            )
            .await?;
        Ok(CreationPlacement {
            parent: request.parent,
            child: request.child,
            descriptor,
            root: resolved.root,
            start,
            _project: project,
            _owner: Arc::clone(self),
            _metadata: metadata,
        })
    }
}

impl CreationPlacement {
    pub(crate) const fn parent(&self) -> SessionId {
        self.parent
    }
    pub(crate) const fn child(&self) -> SessionId {
        self.child
    }
    pub(crate) const fn workspace(&self) -> WorkspaceId {
        self.descriptor.id
    }
    pub(crate) const fn descriptor(&self) -> &WorkspaceDescriptor {
        &self.descriptor
    }
    pub(crate) const fn root(&self) -> &Root {
        &self.root
    }
    pub(crate) const fn start(&self) -> &WorkspaceStartLease {
        &self.start
    }
}
