//! The scheduler may be restarted between any two receipts. Recover the
//! existing actors and bindings before the scheduler can fire another run.

use std::future::Future;
use std::pin::Pin;

use futures_util::stream::{FuturesUnordered, StreamExt as _};
use vessel_proto::registry::WorkspaceResource;
use vessel_proto::{AgentEvent, SessionEvent, SessionStatus};
use vessel_store::{RegistryStartState, RunBindingStage, ScheduleRunBinding};

use super::NodeService;
use super::isolation_cleanup::IsolationCleanup;
use crate::error::ServiceError;
use crate::workspace::{WorkspaceActors as _, WorkspaceError};

type Activity = Pin<Box<dyn Future<Output = WorkspaceResource> + Send>>;

impl NodeService {
    pub(crate) async fn recover_schedule_workspaces(&self) {
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
        for start in store.pending_registry_starts(binding.workspace).await? {
            if start.state != RegistryStartState::Uncertain {
                continue;
            }
            let resource = start.lease.resource();
            if self.live(resource) {
                store.complete_registry_start(start.lease).await?;
            } else {
                // Startup has joined the previous node's process lifetime.
                // No actor survived admission into this runtime and no start
                // may replay its reserved id.
                store.fail_registry_start(start.lease).await?;
            }
        }
        let result = self.dispose_isolation(binding).await?;
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
            let bindings = match store.schedule_run_bindings().await {
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
                    RunBindingStage::Bound
                    | RunBindingStage::Started
                    | RunBindingStage::RetiringArchive
                    | RunBindingStage::RetiringUnpublished => false,
                };
                if ready && let Err(error) = self.cleanup_isolation(binding).await {
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
