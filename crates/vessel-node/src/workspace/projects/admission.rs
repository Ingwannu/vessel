//! Retain project metadata serialization across backing I/O and descriptor commit.
//! This is node metadata admission, not config approval or physical-root exclusion.

use super::super::{WorkspaceError, Workspaces, roots::Resolved};
use std::sync::{Arc, Weak};
use vessel_proto::{ProjectId, workspace::WorkspaceRoot};
use vessel_workspace::paths::Root;

/// Bounds retained admissions and their lock waiters before waiting begins.
pub(crate) const MAX_PROJECT_ADMISSIONS: usize = 64;

/// An opaque hold on the exact registered project and its checked root.
/// The provisioner retains an `Arc` until its independent descriptor is finalized.
#[derive(Debug)]
pub(crate) struct ProjectAdmission {
    owner: Weak<Workspaces>,
    resolved: Resolved,
    _serial: tokio::sync::OwnedMutexGuard<()>,
    _capacity: tokio::sync::OwnedSemaphorePermit,
}

impl ProjectAdmission {
    pub(crate) fn root(&self) -> &Root {
        &self.resolved.root
    }

    pub(crate) fn require(
        &self,
        owner: &Arc<Workspaces>,
        project: ProjectId,
    ) -> Result<(), WorkspaceError> {
        if self.resolved.project != project || !self.owner.ptr_eq(&Arc::downgrade(owner)) {
            return Err(WorkspaceError::Conflict(
                "the project admission belongs to another project or node".to_owned(),
            ));
        }
        Ok(())
    }
}

impl Workspaces {
    /// Resolve a ready project while retaining the project metadata mutation lock.
    /// The caller owns any additional workspace lifecycle/start and root-I/O claims.
    pub(crate) async fn admit_project(
        self: &Arc<Self>,
        project: ProjectId,
    ) -> Result<Arc<ProjectAdmission>, WorkspaceError> {
        let capacity = Arc::clone(&self.project_admissions)
            .try_acquire_owned()
            .map_err(|_full| {
                WorkspaceError::Busy("too many project operations need admission".to_owned())
            })?;
        let serial = Arc::clone(&self.project_registrations).lock_owned().await;
        let resolved = self.resolve(WorkspaceRoot::Project { project }).await?;
        Ok(Arc::new(ProjectAdmission {
            owner: Arc::downgrade(self),
            resolved,
            _serial: serial,
            _capacity: capacity,
        }))
    }
}
