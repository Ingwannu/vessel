//! Snapshot capture precedes file-task admission; physical completion owns
//! before/after cache invalidation, including errors and abandoned delivery.
//!
//! [Decision Log]
//! - Reuse the protected root snapshot, comparison mutation guard and C5's one
//!   retained file-work task/permit. No pool, resolver or controller is added.
//! - Constructors capture registry facts synchronously, before the service
//!   schedules their owned future. Re-resolve that SAME snapshot when polled.
//! - The guard lives inside that retained operation while lower blocking work
//!   is awaited; dropping an RPC waiter cannot drop the guard before disk work.
//! - Non-Git folders stay writable. The Git resolver retains its !git refusal.
//!   Root revalidation is not an atomic filesystem lease against other actors.
use super::{WorkspaceError, Workspaces};
use std::future::Future;
use std::sync::Arc;
use vessel_proto::workspace::WorkspaceRoot;
use vessel_workspace::files::FileError;
use vessel_workspace::paths::Root;

impl Workspaces {
    /// Construct before service admission, then retain the future through the
    /// lower disk operation. Reply cancellation must never own this lifetime.
    pub(in crate::workspace) fn mutating_file<T, F, Work>(
        self: &Arc<Self>,
        root: WorkspaceRoot,
        work: F,
    ) -> impl Future<Output = Result<T, WorkspaceError>> + Send + 'static
    where
        T: Send + 'static,
        F: FnOnce(Root) -> Work + Send + 'static,
        Work: Future<Output = Result<T, FileError>> + Send + 'static,
    {
        let admission = self.root_snapshot(root);
        let owner = Arc::downgrade(self);
        async move {
            let owner = owner
                .upgrade()
                .ok_or(crate::workspace::comparison::JobError::Closed)?;
            if owner.stop.is_cancelled() {
                return Err(crate::workspace::comparison::JobError::Closed.into());
            }
            let admission = admission?;
            let resolved = owner.resolve_unchanged_root(root, &admission).await?;
            if owner.stop.is_cancelled() {
                return Err(crate::workspace::comparison::JobError::Closed.into());
            }
            let _mutation = owner.comparisons.mutating(
                root,
                resolved.project,
                crate::workspace::comparison::Change::Working,
            );
            work(resolved.root).await.map_err(WorkspaceError::from)
        }
    }
}
