//! Read-only preview publication shares capture's real worker and root admission.
//!
//! [Decision Log]
//! - Purpose: confirmation must describe a frozen, principal-bound desired image.
//! - Constraint: a caller timeout cannot stop arbitrary filesystem reads or fsync.
//! - Choice: the tracker owns deadline cancellation and actual cleanup, retaining
//!   the shared worker/root/SQL escrow. Submitted SQL may commit after RPC timeout.
//! - Cost: uncertain SQL/cleanup keeps capacity unavailable until reconciliation.
use super::{Controller, backend::reason, workers::WorkerLease};
use crate::workspace::PreviewRead;
use std::sync::Arc;
use tokio::{sync::oneshot, time::Instant};
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{
    CHECKPOINT_CAPTURE_DEADLINE, CHECKPOINT_PREVIEW_LIFETIME, CaptureKey, CheckpointError,
    CheckpointOwner, PathCount, PreviewCheckpoint, RestorePlanId, RestorePreview,
};
use vessel_store::checkpoint::{Allocation, CheckpointWrite, Plan};

impl Controller {
    pub(crate) async fn preview(
        &self,
        owner: CheckpointOwner,
        request: PreviewCheckpoint,
    ) -> Result<RestorePreview, CheckpointError> {
        let deadline = Instant::now()
            .checked_add(CHECKPOINT_CAPTURE_DEADLINE)
            .ok_or(CheckpointError::CaptureTimedOut)?;
        let slot = Arc::clone(&self.jobs)
            .try_acquire_owned()
            .map_err(|_busy| CheckpointError::Busy)?;
        let lease = self.context.workers().claim()?;
        let cancel = self.context.stop.child_token();
        let task_cancel = cancel.clone();
        let controller = self.clone();
        let (reply, received) = oneshot::channel();
        let _job = self.context.tasks.spawn(async move {
            let _slot = slot;
            let work = controller.make_preview(owner, request, deadline, &task_cancel, &lease);
            let result = super::workers::cancel_at(deadline, &task_cancel, work).await;
            let _abandoned = reply.send(result);
        });
        match tokio::time::timeout_at(deadline, received).await {
            Ok(Ok(result)) => result,
            Ok(Err(_gone)) => Err(CheckpointError::Unknown),
            Err(_deadline) => {
                cancel.cancel();
                Err(CheckpointError::CaptureTimedOut)
            }
        }
    }
    async fn make_preview(
        &self,
        owner: CheckpointOwner,
        request: PreviewCheckpoint,
        deadline: Instant,
        cancel: &CancellationToken,
        lease: &Arc<WorkerLease>,
    ) -> Result<RestorePreview, CheckpointError> {
        let (selected, guard) = self.preview_selection(request, deadline).await?;
        let defaults = self.context.defaults.clone()?;
        let key = selected.key;
        let plan = RestorePlanId::random();
        self.allocate_preview(key, plan, owner, lease).await?;
        let workspaces = Arc::clone(&self.context.workspaces);
        let runtime = tokio::runtime::Handle::current();
        let actual = Arc::clone(lease);
        let stopped = cancel.clone();
        let image = selected.snapshot.image().clone();
        let physical = guard.physical();
        let result = tokio::task::spawn_blocking(move || {
            let _worker = actual;
            workspaces.preview_checkpoint(
                &runtime,
                &PreviewRead {
                    key,
                    physical,
                    selected: selected.snapshot,
                    plan,
                },
                &defaults,
                &stopped,
            )
        })
        .await
        .map_err(|_join| CheckpointError::Unknown)
        .and_then(std::convert::identity);
        let current = match result {
            Ok(current) if Instant::now() < deadline && !cancel.is_cancelled() => current,
            Ok(_) => {
                self.discard_preview(key, plan, owner, lease).await?;
                return Err(CheckpointError::CaptureTimedOut);
            }
            Err(failure) => {
                self.discard_preview(key, plan, owner, lease).await?;
                return Err(failure);
            }
        };
        let prepared = freeze(key, plan, owner, &image, &current);
        let (frozen, changed) = match prepared {
            Ok(prepared) => prepared,
            Err(failure) => {
                self.discard_preview(key, plan, owner, lease).await?;
                return Err(failure);
            }
        };
        let result = self.publish_preview(frozen, changed, lease).await;
        // Actual read I/O and durable publication finished before root release.
        drop(guard);
        result
    }
    async fn allocate_preview(
        &self,
        key: CaptureKey,
        plan: RestorePlanId,
        owner: CheckpointOwner,
        lease: &Arc<WorkerLease>,
    ) -> Result<(), CheckpointError> {
        if let Err(error) = self
            .context
            .store
            .checkpoint_write(CheckpointWrite::Allocate {
                key,
                allocation: Allocation::Preview { plan },
                owner,
                at: chrono::Utc::now(),
            })
            .await
        {
            let refusal = reason(error);
            // These writer refusals precede insertion. Other failures may have
            // committed and must retain both persistent escrow and worker budget.
            if !matches!(
                refusal,
                CheckpointError::StorageFull
                    | CheckpointError::StaleCapture
                    | CheckpointError::RestoreIdConflict
            ) {
                self.hold(lease).await;
            }
            return Err(refusal);
        }
        Ok(())
    }
    async fn preview_selection(
        &self,
        request: PreviewCheckpoint,
        deadline: Instant,
    ) -> Result<
        (
            vessel_store::checkpoint::Selected,
            crate::workspace::StartGuard,
        ),
        CheckpointError,
    > {
        tokio::time::timeout_at(deadline, async {
            let _visible = self
                .context
                .sessions
                .get(request.session)
                .await
                .map_err(|_missing| CheckpointError::NotFound)?;
            let selected = self
                .context
                .store
                .checkpoint_selected(request.session, request.checkpoint)
                .await
                .map_err(reason)?;
            let guard = self
                .context
                .workspaces
                .checkpoint_admission(selected.key.worktree)
                .await?;
            if guard.physical() != selected.snapshot.evidence().physical() {
                return Err(CheckpointError::StaleCapture);
            }
            guard
                .revalidate()
                .await
                .map_err(|_changed| CheckpointError::StaleCapture)?;
            Ok::<_, CheckpointError>((selected, guard))
        })
        .await
        .map_err(|_deadline| CheckpointError::CaptureTimedOut)?
    }
    async fn publish_preview(
        &self,
        frozen: Plan,
        changed: PathCount,
        lease: &Arc<WorkerLease>,
    ) -> Result<RestorePreview, CheckpointError> {
        let key = frozen.key;
        let plan = frozen.id;
        let owner = frozen.owner;
        let allocation = Allocation::Preview { plan };
        if let Err(error) = self
            .context
            .store
            .checkpoint_write(CheckpointWrite::Plan {
                plan: Box::new(frozen),
                at: chrono::Utc::now(),
            })
            .await
        {
            // Publication may have committed. Never erase a referenced archive.
            self.hold(lease).await;
            return Err(reason(error));
        }
        if let Err(error) = self
            .context
            .store
            .checkpoint_write(CheckpointWrite::AllocationAttached {
                key,
                allocation,
                owner,
            })
            .await
        {
            self.hold(lease).await;
            return Err(reason(error));
        }
        let page = self
            .context
            .store
            .checkpoint_changes(plan, owner, None, chrono::Utc::now())
            .await
            .map_err(reason)?;
        Ok(RestorePreview {
            boundary: key.boundary,
            generation: key.generation,
            changed,
            page,
        })
    }
    pub(super) async fn hold(&self, lease: &Arc<WorkerLease>) {
        self.held.lock().await.push(Arc::clone(lease));
    }
    async fn discard_preview(
        &self,
        key: CaptureKey,
        plan: RestorePlanId,
        owner: CheckpointOwner,
        lease: &Arc<WorkerLease>,
    ) -> Result<(), CheckpointError> {
        let workspaces = Arc::clone(&self.context.workspaces);
        let actual = Arc::clone(lease);
        let cleanup = tokio::task::spawn_blocking(move || {
            let _worker = actual;
            workspaces.clean_checkpoint_preview(key, plan)
        })
        .await
        .map_err(|_join| CheckpointError::Unknown)
        .and_then(std::convert::identity);
        if let Err(failure) = cleanup {
            self.hold(lease).await;
            return Err(failure);
        }
        if let Err(error) = self
            .context
            .store
            .checkpoint_write(CheckpointWrite::AllocationCleaned {
                key,
                allocation: Allocation::Preview { plan },
                owner,
            })
            .await
        {
            self.hold(lease).await;
            return Err(reason(error));
        }
        Ok(())
    }
}

fn freeze(
    key: CaptureKey,
    plan: RestorePlanId,
    owner: CheckpointOwner,
    image: &vessel_proto::checkpoint::FileImage,
    current: &vessel_proto::checkpoint::ObservedSnapshot,
) -> Result<(Plan, PathCount), CheckpointError> {
    let desired = image.effective(
        current.image(),
        vessel_workspace::checkpoint::private_metadata()?,
    )?;
    let changed = PathCount::new(desired.changes(current.image()).len())?;
    let expires = chrono::Utc::now()
        .checked_add_signed(
            chrono::Duration::from_std(CHECKPOINT_PREVIEW_LIFETIME)
                .map_err(|_overflow| CheckpointError::Unknown)?,
        )
        .ok_or(CheckpointError::Unknown)?;
    Ok::<_, CheckpointError>((
        Plan {
            id: plan,
            key,
            owner,
            current: current.image().clone(),
            evidence: current.evidence().clone(),
            desired,
            expires,
        },
        changed,
    ))
}
