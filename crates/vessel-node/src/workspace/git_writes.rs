//! Serialize Git RPC mutations for one registered project, including worktrees.
//!
//! [Decision Log]
//! - Purpose: a multi-command merge must not temporarily expose its checkout to
//!   another Vessel commit/switch request from the same repository.
//! - Existing assets: authoritative project ids, root revalidation, bounded
//!   Tokio admission and the node cancellation token. No path/alias registry.
//! - Choice: weak per-project mutexes under a short coordination-only map lock;
//!   64 permits bound waiting plus executing writers. Re-resolve after waiting.
//! - Why: Git index.lock covers one command, not checkout/merge/restore. Weak
//!   entries do not retain closed projects. Other projects remain independent.
//! - Limits: this serializes Git RPC writes, not external CLI/agent processes
//!   or project/worktree lifecycle operations. It is not a filesystem sandbox.
use super::{WorkspaceError, Workspaces, roots::Resolved};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError, Weak};
use tokio::sync::{
    Mutex as AsyncMutex, OwnedMutexGuard, OwnedSemaphorePermit, Semaphore, TryAcquireError,
};
use tokio_util::sync::CancellationToken;
use vessel_proto::ProjectId;
use vessel_proto::workspace::WorkspaceRoot;

const MAX_WRITERS: usize = 64;
#[derive(Debug)]
pub(super) struct Writers {
    projects: Mutex<HashMap<ProjectId, Weak<AsyncMutex<()>>>>,
    admitted: Arc<Semaphore>,
    #[cfg(test)]
    pub(super) waiting: tokio::sync::Notify,
}
impl Default for Writers {
    fn default() -> Self {
        Self {
            projects: Mutex::default(),
            admitted: Arc::new(Semaphore::new(MAX_WRITERS)),
            #[cfg(test)]
            waiting: tokio::sync::Notify::new(),
        }
    }
}
/// Admission and serialization are both held through the caller's operation.
#[derive(Debug)]
pub(super) struct Lease {
    _admission: OwnedSemaphorePermit,
    _project: OwnedMutexGuard<()>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum WriteAdmission {
    #[error("too many Git changes are waiting; try again")]
    Busy,
    #[error("the node is stopping; no Git change was started")]
    Closed,
}
impl Writers {
    async fn acquire(
        &self,
        project: ProjectId,
        stop: &CancellationToken,
    ) -> Result<Lease, WriteAdmission> {
        if stop.is_cancelled() {
            return Err(WriteAdmission::Closed);
        }
        let admission =
            Arc::clone(&self.admitted)
                .try_acquire_owned()
                .map_err(|error| match error {
                    TryAcquireError::Closed => WriteAdmission::Closed,
                    TryAcquireError::NoPermits => WriteAdmission::Busy,
                })?;
        let serial = {
            // Only Weak coordination handles live here. Recovering this map
            // after a panic cannot recover business/authorization facts or
            // bypass admission; every target is re-resolved below the lease.
            let mut projects = self.projects.lock().unwrap_or_else(PoisonError::into_inner);
            projects.retain(|_, entry| entry.strong_count() > 0);
            if let Some(active) = projects.get(&project).and_then(Weak::upgrade) {
                active
            } else {
                let active = Arc::new(AsyncMutex::new(()));
                let _old = projects.insert(project, Arc::downgrade(&active));
                active
            }
        };
        #[cfg(test)]
        self.waiting.notify_waiters();
        let project = tokio::select! {
            biased;
            () = stop.cancelled() => return Err(WriteAdmission::Closed),
            guard = serial.lock_owned() => guard,
        };
        Ok(Lease {
            _admission: admission,
            _project: project,
        })
    }
}
/// The freshly validated target and its inseparable mutation reservation.
#[derive(Debug)]
pub(super) struct WriteRoot {
    pub(super) target: Resolved,
    _lease: Lease,
}
impl Workspaces {
    pub(super) async fn locked_git(
        &self,
        root: WorkspaceRoot,
    ) -> Result<WriteRoot, WorkspaceError> {
        let _git = self.git()?;
        let before = self.root_snapshot(root)?;
        if !before.git {
            return Err(WorkspaceError::NotGit);
        }
        let lease = self.git_writers.acquire(before.project, &self.stop).await?;
        let current = self.resolve_unchanged(root, &before).await?;
        Ok(WriteRoot {
            target: current,
            _lease: lease,
        })
    }
}

#[cfg(test)]
mod tests;
