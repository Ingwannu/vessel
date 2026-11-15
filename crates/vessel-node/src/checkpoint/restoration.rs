//! Owned restore jobs outlive RPC cancellation; existing ids never reapply files.
use super::{Controller, backend::reason, bridge, execution::Execution, workers::WorkerLease};
use std::sync::Arc;
use tokio::{
    sync::{oneshot, watch},
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use vessel_proto::checkpoint::{
    CHECKPOINT_RESTORE_DEADLINE, CheckpointError, CheckpointOwner, RestoreCheckpoint,
    RestoreOutcome, StagedPaths,
};
use vessel_store::checkpoint::{Allocation, CheckpointWrite, Operation, Plan};

#[derive(Clone, Copy)]
enum Progress {
    Pending,
    Admitted,
}
struct Job {
    owner: CheckpointOwner,
    request: RestoreCheckpoint,
    deadline: Instant,
    lease: Arc<WorkerLease>,
    cancel: CancellationToken,
    progress: watch::Sender<Progress>,
}
impl Controller {
    pub(crate) async fn restore(
        &self,
        owner: CheckpointOwner,
        request: RestoreCheckpoint,
    ) -> Result<RestoreOutcome, CheckpointError> {
        // Idempotent terminal/in-flight retries precede expiry and admission.
        if let Some(operation) = self.existing(owner, request).await? {
            if operation.plan != request.plan {
                return Err(CheckpointError::RestoreIdConflict);
            }
            if !matches!(operation.outcome, RestoreOutcome::RecoveryRequired { .. }) {
                if matches!(operation.outcome, RestoreOutcome::Restored { .. }) {
                    self.publish_ended(&operation).await?;
                }
                return Ok(operation.outcome);
            }
        }
        // No legacy terminal-list/ended/cwd/PID substitute is acceptable. The
        // production quiescence producer is still unavailable; this stays Busy.
        self.terminal_quiescence()?;
        let deadline = Instant::now()
            .checked_add(CHECKPOINT_RESTORE_DEADLINE)
            .ok_or(CheckpointError::Busy)?;
        let slot = Arc::clone(&self.jobs)
            .try_acquire_owned()
            .map_err(|_busy| CheckpointError::Busy)?;
        let lease = self.context.workers().claim()?;
        let cancel = self.context.stop.child_token();
        let (progress, watching) = watch::channel(Progress::Pending);
        let job = Job {
            owner,
            request,
            deadline,
            lease,
            cancel: cancel.clone(),
            progress,
        };
        let controller = self.clone();
        let (reply, received) = oneshot::channel();
        let _job = self.context.tasks.spawn(async move {
            let _slot = slot;
            let deadline = job.deadline;
            let cancellation = job.cancel.clone();
            let work = controller.run_restore(job);
            let result = super::workers::cancel_at(deadline, &cancellation, work).await;
            let _abandoned = reply.send(result);
        });
        match tokio::time::timeout_at(deadline, received).await {
            Ok(Ok(result)) => result,
            Ok(Err(_lost)) => Err(CheckpointError::Unknown),
            Err(_deadline) => {
                cancel.cancel();
                match *watching.borrow() {
                    Progress::Pending => Err(CheckpointError::CaptureTimedOut),
                    Progress::Admitted => Ok(RestoreOutcome::InProgress {
                        restore: request.restore,
                    }),
                }
            }
        }
    }
    async fn run_restore(&self, job: Job) -> Result<RestoreOutcome, CheckpointError> {
        let (plan, guard, recovering) = self.restore_scope(&job).await?;
        let (files, recovering) = self.observe_restore(&plan, recovering, &job).await?;
        let operation = self.admit_restore(&plan, &guard, recovering, &job).await?;
        let _prior = job.progress.send_replace(Progress::Admitted);
        let execution = self.run_io(&plan, &operation, files, &job).await;
        self.finish_restore(&plan, &operation, guard, execution)
            .await
    }
    async fn restore_scope(
        &self,
        job: &Job,
    ) -> Result<(Plan, crate::workspace::StartGuard, Option<Operation>), CheckpointError> {
        let (plan, consumed) = self
            .context
            .store
            .checkpoint_plan(job.request.plan, job.owner)
            .await
            .map_err(reason)?;
        if plan.key.session != job.request.session {
            return Err(CheckpointError::Forbidden);
        }
        let existing = self.existing(job.owner, job.request).await?;
        let recovering = match existing.as_ref() {
            Some(operation) if operation.plan != plan.id => {
                return Err(CheckpointError::RestoreIdConflict);
            }
            Some(operation)
                if matches!(operation.outcome, RestoreOutcome::RecoveryRequired { .. }) =>
            {
                Some(operation.clone())
            }
            Some(_operation) => return Err(CheckpointError::Busy),
            None => None,
        };
        if recovering.is_none() && (consumed.is_some() || plan.expires <= chrono::Utc::now()) {
            return Err(CheckpointError::StalePreview);
        }
        let guard = if recovering.is_some() {
            self.context
                .workspaces
                .checkpoint_recovery_admission(
                    plan.key.worktree,
                    plan.evidence.physical(),
                    job.request.restore,
                )
                .await?
        } else {
            self.context
                .workspaces
                .checkpoint_admission(plan.key.worktree)
                .await?
        };
        if guard.physical() != plan.evidence.physical() {
            return Err(CheckpointError::StalePreview);
        }
        self.context
            .sessions
            .checkpoint_ended_target(plan.key.session, guard.physical())
            .await?;
        guard
            .revalidate()
            .await
            .map_err(|_changed| CheckpointError::StalePreview)?;
        Ok((plan, guard, recovering))
    }
    async fn observe_restore(
        &self,
        plan: &Plan,
        recovering: Option<Operation>,
        job: &Job,
    ) -> Result<(crate::workspace::CheckpointFiles, Option<Operation>), CheckpointError> {
        let defaults = bridge::defaults()?;
        let workspaces = Arc::clone(&self.context.workspaces);
        let read_plan = plan.clone();
        let read_runtime = tokio::runtime::Handle::current();
        let read_defaults = defaults.clone();
        let actual = Arc::clone(&job.lease);
        let cancel = job.cancel.clone();
        let files = tokio::task::spawn_blocking(move || {
            let _worker = actual;
            let files =
                workspaces.checkpoint_files(read_plan.key, read_plan.evidence.physical())?;
            if recovering.is_none() {
                let observed =
                    files.observe(&read_runtime, &read_defaults, &read_plan.current, &cancel)?;
                if observed.image() != &read_plan.current
                    || observed.evidence() != &read_plan.evidence
                {
                    return Err(CheckpointError::StalePreview);
                }
            }
            Ok::<_, CheckpointError>((files, recovering))
        })
        .await
        .map_err(|_join| CheckpointError::Unknown)??;
        Ok(files)
    }
    async fn admit_restore(
        &self,
        plan: &Plan,
        guard: &crate::workspace::StartGuard,
        recovering: Option<Operation>,
        job: &Job,
    ) -> Result<Operation, CheckpointError> {
        if let Some(operation) = recovering {
            Ok(operation)
        } else {
            if job.cancel.is_cancelled() || Instant::now() >= job.deadline {
                return Err(CheckpointError::CaptureTimedOut);
            }
            if plan.expires <= chrono::Utc::now() {
                return Err(CheckpointError::StalePreview);
            }
            guard
                .pin_restore(plan.key.worktree, job.request.restore)
                .await?;
            match self
                .context
                .store
                .checkpoint_write(CheckpointWrite::Admit {
                    plan: Box::new(plan.clone()),
                    restore: job.request.restore,
                    at: chrono::Utc::now(),
                })
                .await
            {
                Ok(()) => {}
                Err(error) => {
                    let refusal = reason(error);
                    if matches!(
                        refusal,
                        CheckpointError::StorageFull
                            | CheckpointError::StalePreview
                            | CheckpointError::RestoreIdConflict
                    ) {
                        guard.settle_restore(job.request.restore).await?;
                    } else {
                        self.hold(&job.lease).await;
                    }
                    return Err(refusal);
                }
            }
            self.context
                .store
                .checkpoint_operation(plan.key.session, job.request.restore, job.owner)
                .await
                .map_err(reason)
        }
    }
    async fn run_io(
        &self,
        plan: &Plan,
        operation: &Operation,
        files: crate::workspace::CheckpointFiles,
        job: &Job,
    ) -> Result<RestoreOutcome, CheckpointError> {
        let store = self.context.store.clone();
        let actual = Arc::clone(&job.lease);
        let mutate_plan = plan.clone();
        let mutate_operation = operation.clone();
        let stopped = job.cancel.clone();
        let runtime = tokio::runtime::Handle::current();
        let defaults = bridge::defaults()?;
        tokio::task::spawn_blocking(move || {
            let _worker = actual;
            let execute = Execution {
                runtime: &runtime,
                store: &store,
                plan: &mutate_plan,
                operation: &mutate_operation,
                defaults: &defaults,
                cancel: &stopped,
            };
            if matches!(
                mutate_operation.outcome,
                RestoreOutcome::RecoveryRequired { .. }
            ) {
                let paths = runtime
                    .block_on(store.checkpoint_paths(mutate_operation.clone()))
                    .map_err(reason)?;
                let staged = paths
                    .into_iter()
                    .map(|intent| intent.staged)
                    .collect::<Option<Vec<_>>>();
                let restored = match staged {
                    Some(staged) if !staged.is_empty() => Some(files.recovering(
                        &mutate_plan,
                        mutate_operation.id,
                        &StagedPaths::new(staged)?,
                    )?),
                    Some(_) | None => None,
                };
                execute.rollback(&files, restored.as_ref(), CheckpointError::RecoveryPending)
            } else if mutate_plan.expires <= chrono::Utc::now() || stopped.is_cancelled() {
                execute.rollback(&files, None, CheckpointError::StalePreview)
            } else {
                execute.run(&files)
            }
        })
        .await
        .map_err(|_join| CheckpointError::Unknown)
        .and_then(std::convert::identity)
    }
    async fn finish_restore(
        &self,
        plan: &Plan,
        operation: &Operation,
        guard: crate::workspace::StartGuard,
        execution: Result<RestoreOutcome, CheckpointError>,
    ) -> Result<RestoreOutcome, CheckpointError> {
        let outcome = match execution {
            Ok(outcome) => outcome,
            Err(_failure) => {
                // Actual I/O returned. Reload an uncertain terminal commit before
                // deciding whether recovery is still required.
                let latest = self
                    .context
                    .store
                    .checkpoint_operation(operation.session, operation.id, operation.owner)
                    .await
                    .map_err(reason)?;
                match latest.outcome {
                    RestoreOutcome::InProgress { .. } => {
                        self.context
                            .store
                            .checkpoint_write(CheckpointWrite::Outcome {
                                operation: Box::new(latest),
                                result: RestoreOutcome::RecoveryRequired {
                                    restore: operation.id,
                                },
                            })
                            .await
                            .map_err(reason)?;
                        self.attach_restore(plan, operation).await?;
                        return Ok(RestoreOutcome::RecoveryRequired {
                            restore: operation.id,
                        });
                    }
                    RestoreOutcome::RecoveryRequired { .. } => {
                        self.attach_restore(plan, operation).await?;
                        return Ok(latest.outcome);
                    }
                    terminal => terminal,
                }
            }
        };
        self.attach_restore(plan, operation).await?;
        let finished = self
            .context
            .store
            .checkpoint_operation(operation.session, operation.id, operation.owner)
            .await
            .map_err(reason)?;
        if matches!(outcome, RestoreOutcome::Restored { .. }) {
            self.publish_ended(&finished).await?;
        }
        guard
            .revalidate()
            .await
            .map_err(|_changed| CheckpointError::RecoveryPending)?;
        guard.settle_restore(operation.id).await?;
        drop(guard);
        Ok(outcome)
    }
    async fn existing(
        &self,
        owner: CheckpointOwner,
        request: RestoreCheckpoint,
    ) -> Result<Option<Operation>, CheckpointError> {
        match self
            .context
            .store
            .checkpoint_operation(request.session, request.restore, owner)
            .await
        {
            Ok(operation) => Ok(Some(operation)),
            Err(error) => match reason(error) {
                CheckpointError::NotFound => Ok(None),
                refusal => Err(refusal),
            },
        }
    }
    async fn attach_restore(
        &self,
        plan: &Plan,
        operation: &Operation,
    ) -> Result<(), CheckpointError> {
        self.context
            .store
            .checkpoint_write(CheckpointWrite::AllocationAttached {
                key: plan.key,
                allocation: Allocation::Restore {
                    restore: operation.id,
                },
                owner: operation.owner,
            })
            .await
            .map_err(reason)
    }
    async fn publish_ended(&self, operation: &Operation) -> Result<(), CheckpointError> {
        if self
            .context
            .store
            .checkpoint_event_seq(operation.clone())
            .await
            .map_err(reason)?
            .is_some()
        {
            return Ok(());
        }
        let info = self
            .context
            .sessions
            .get(operation.session)
            .await
            .map_err(|_missing| CheckpointError::NotFound)?;
        if info.status.is_live() {
            return Err(CheckpointError::Busy);
        }
        self.context
            .store
            .checkpoint_write(CheckpointWrite::EndedMetadata {
                operation: Box::new(operation.clone()),
                at: chrono::Utc::now(),
            })
            .await
            .map_err(reason)?;
        if self
            .context
            .store
            .checkpoint_event_seq(operation.clone())
            .await
            .map_err(reason)?
            .is_none()
        {
            return Err(CheckpointError::RecoveryPending);
        }
        Ok(())
    }
}
