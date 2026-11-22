//! The scheduler may be restarted between any two receipts. Recover the
//! existing actors and bindings before the scheduler can fire another run.

use std::future::Future;
use std::pin::Pin;

use futures_util::stream::{FuturesUnordered, StreamExt as _};
use vessel_proto::automation::AfterRun;
use vessel_proto::registry::{WorkspaceLifecycle, WorkspaceResource};
use vessel_proto::{AgentEvent, SessionEvent, SessionStatus};
use vessel_store::{RunBindingStage, ScheduleRunBinding};

use super::NodeService;
use super::isolation_cleanup::IsolationCleanup;
use crate::error::ServiceError;
use crate::workspace::WorkspaceError;

type Activity = Pin<Box<dyn Future<Output = WorkspaceResource> + Send>>;

impl NodeService {
    pub(crate) async fn recover_schedule_workspaces(&self) {
        self.workspaces.resume_explicit_archives(self).await;
        let store = self.sessions.store();
        let bindings = match store.schedule_run_bindings().await {
            Ok(bindings) => bindings,
            Err(error) => {
                tracing::error!(%error, "cannot read isolated run recovery receipts");
                return;
            }
        };
        for binding in bindings {
            if let Err(error) = self.recover_schedule_workspace(binding).await {
                tracing::warn!(workspace = %binding.workspace, %error, "isolated run recovery still owes work");
                if binding.stage.retirement().is_none()
                    && binding.stage != RunBindingStage::Unconfirmed
                    && let Err(record) = store
                        .advance_schedule_run_binding(binding, RunBindingStage::Unconfirmed)
                        .await
                {
                    tracing::warn!(workspace = %binding.workspace, %record, "cannot retain an unconfirmed isolation receipt");
                }
            }
        }
        self.workspaces.retire_reconciled_archives().await;
    }

    async fn recover_schedule_workspace(
        &self,
        binding: ScheduleRunBinding,
    ) -> Result<(), WorkspaceError> {
        if let Some(retirement) = binding.stage.retirement() {
            return self
                .settle_isolation(
                    binding,
                    IsolationCleanup::Settled {
                        archive: None,
                        retirement: Some(retirement),
                    },
                )
                .await;
        }
        let store = self.sessions.store();
        let keep = binding.after == AfterRun::Keep
            && matches!(binding.stage, RunBindingStage::Started | RunBindingStage::Kept);
        let releasing = keep && store.schedule_keeps_session(binding.session, true).await?;
        if keep && !releasing && binding.stage == RunBindingStage::Kept {
            return Ok(());
        }
        if (!keep || releasing)
            && store.registry_workspace(binding.workspace).await?.is_some_and(|record| {
                !matches!(record.descriptor.lifecycle, WorkspaceLifecycle::Archived { .. })
            })
        {
            self.workspaces.reconcile_workspace_starts(binding.workspace, self).await?;
        }
        let result = if releasing {
            self.archive_isolation(binding).await?
        } else {
            self.dispose_isolation(binding).await?
        };
        if let Some(archive) = result.disposition()
            && let Some(mut run) = store
                .runs(binding.schedule)
                .await?
                .into_iter()
                .find(|run| run.number == binding.run)
            && run.archive.is_none()
        {
            run.archive = Some(archive);
            let schedule = store
                .automation()
                .await?
                .schedules
                .into_iter()
                .find(|info| info.id == binding.schedule);
            match schedule {
                Some(mut schedule)
                    if schedule
                        .last_run
                        .as_ref()
                        .is_some_and(|last| last.number == run.number) =>
                {
                    schedule.last_run = Some(run.clone());
                    store.put_schedule(schedule, Some(run)).await?;
                }
                Some(_) => store.put_run(binding.schedule, run).await?,
                None => {}
            }
        }
        self.settle_isolation(binding, result).await
    }

    /// Deferred claims wake on exact member progress or committed registry
    /// changes. The observer never treats its own defer write as progress.
    pub(crate) async fn watch_deferred_schedule_workspaces(&self) {
        let stopping = self.lifecycle.stopping();
        let store = self.sessions.store();
        let mut changes = store.registry_changes();
        loop {
            let bindings = match store.schedule_bindings_awaiting_archive().await {
                Ok(bindings) => bindings,
                Err(error) => {
                    tracing::warn!(%error, "cannot watch deferred workspace archives");
                    return;
                }
            };
            let mut observations = Vec::new();
            for binding in bindings {
                let ready = match binding.stage {
                    RunBindingStage::DeferredBusy => {
                        self.watch_busy_archive(binding, &mut observations).await
                    }
                    RunBindingStage::DeferredMembers => {
                        self.watch_other_members(binding, &mut observations).await
                    }
                    RunBindingStage::Kept => true,
                    RunBindingStage::Bound
                    | RunBindingStage::Started
                    | RunBindingStage::RetiringArchive
                    | RunBindingStage::RetiringUnpublished
                    | RunBindingStage::Unconfirmed => false,
                };
                if ready && let Err(error) = self.archive_requested_run(binding).await {
                    tracing::warn!(workspace = %binding.workspace, %error, "deferred workspace archive remains unsettled");
                }
            }
            let mut activity = observations.into_iter().collect::<FuturesUnordered<_>>();
            tokio::select! {
                () = stopping.cancelled() => return,
                result = changes.changed() => {
                    if result.is_err() {
                        return;
                    }
                }
                Some(resource) = activity.next(), if !activity.is_empty() => {
                    if let Err(error) = store.notify_registry_member(resource).await {
                        tracing::debug!(%error, "cannot publish deferred workspace activity");
                    }
                }
            }
        }
    }

    async fn archive_requested_run(&self, binding: ScheduleRunBinding) -> Result<(), WorkspaceError> {
        let cleanup = self.archive_isolation(binding).await?;
        if let Some(disposition) = cleanup.disposition() {
            self.schedules.record_archive(binding.schedule, binding.run, disposition).await
                .map_err(|error| WorkspaceError::Conflict(error.to_string()))?;
        }
        self.settle_isolation(binding, cleanup).await
    }

    async fn watch_busy_archive(
        &self,
        binding: ScheduleRunBinding,
        activity: &mut Vec<Activity>,
    ) -> bool {
        let session = binding.session;
        if let Some(handle) = self.sessions.live(session) {
            let resource = WorkspaceResource::Session { session };
            if let Ok(records) = handle.follow(handle.log_end()) {
                activity.push(Box::pin(async move {
                    let mut live = records.live;
                    if !records
                        .cached
                        .iter()
                        .any(|record| idle_activity(&record.event))
                    {
                        while let Ok(record) = live.recv().await {
                            if idle_activity(&record.event) {
                                break;
                            }
                        }
                    }
                    resource
                }));
            }
            handle.info().status == SessionStatus::Running
                && handle.conversation().is_ok_and(|current| {
                    current.queue.entries.is_empty()
                        && current.permissions.is_empty()
                        && current.questions.is_empty()
                })
                && handle.turn_running().await.is_ok_and(|running| !running)
        } else {
            if self.sessions.get(session).await.is_ok_and(|info| info.marks.archived_at.is_some()) {
                return true;
            }
            match self.sessions.conversation(session).await {
                Ok(current) => current.queue.entries.is_empty(),
                Err(ServiceError::NotFound(_)) => true,
                Err(_) => false,
            }
        }
    }

    async fn watch_other_members(
        &self,
        binding: ScheduleRunBinding,
        activity: &mut Vec<Activity>,
    ) -> bool {
        let Ok(members) = self
            .sessions
            .store()
            .registry_members(binding.workspace)
            .await
        else {
            return false;
        };
        let mut ready = true;
        for member in members {
            let resource = member.resource;
            match resource {
                WorkspaceResource::Session { session } if session != binding.session => {
                    if let Some(handle) = self.sessions.live(session) {
                        ready = false;
                        activity.push(Box::pin(async move {
                            let _ended = handle.ended().await;
                            resource
                        }));
                    }
                }
                WorkspaceResource::Terminal { terminal } => {
                    if let Ok(entry) = self.terminals.get(terminal)
                        && entry.handle.info().is_live()
                    {
                        ready = false;
                        activity.push(Box::pin(async move {
                            let _ended = entry.handle.ended().await;
                            resource
                        }));
                    }
                }
                WorkspaceResource::Session { .. } => {}
            }
        }
        ready
    }
}

fn idle_activity(event: &SessionEvent) -> bool {
    matches!(
        event,
        SessionEvent::QueueChanged { .. }
            | SessionEvent::Status { .. }
            | SessionEvent::Agent {
                event: AgentEvent::TurnEnded { .. } | AgentEvent::TurnFailed { .. }
            }
    )
}
