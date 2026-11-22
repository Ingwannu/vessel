//! Schedule admission and run identity. Workspace effects stay in its lifecycle
//! module so other callers can use the same producer and archive claims.

use chrono::Utc;
use vessel_proto::automation::{AfterRun, RunOutcome};
use vessel_proto::registry::{WorkspaceAutomation, WorkspaceIsolation, WorkspaceResource, WorkspaceVisibility};
use vessel_proto::workspace::SessionPlace;
use vessel_proto::{ContentBlock, Delivery, ErrorCode, Method, ProjectId, SessionId, SessionSpec, WireError, WorkspaceId};
use vessel_store::{RunBindingStage, ScheduleRunBinding, WorkspaceProvision};

use super::{NodeService, wire};
use super::scheduled::failed;
use crate::automation::schedules::{Job, Made};
use crate::principal::Principal;
use crate::profiles::launch::creator;
use crate::workspace::Provisioned;

impl NodeService {
    /// Explicit isolation selects a registered root. Existing legacy placements
    /// keep their original rules until an explicit policy is saved.
    pub(super) async fn isolation_project(
        &self,
        principal: &Principal,
        spec: &SessionSpec,
        isolation: &WorkspaceIsolation,
    ) -> Result<ProjectId, WireError> {
        let invalid = |message| WireError::new(ErrorCode::InvalidRequest, message);
        let Some(SessionPlace::Project { project, dir: None }) = spec.place.as_deref() else {
            return Err(invalid("workspace isolation requires a registered project root"));
        };
        if spec.cwd.is_some() {
            return Err(invalid("workspace isolation takes its folder from the project"));
        }
        match isolation {
            WorkspaceIsolation::Local => {}
            WorkspaceIsolation::Worktree { .. } => {
                principal.permit(Method::CreateWorktree)?;
                let info = self
                    .workspaces
                    .get_project(*project, None, false)
                    .await
                    .map_err(|error| error.wire())?;
                if !info.git {
                    return Err(invalid("Worktree isolation requires a Git project"));
                }
            }
            WorkspaceIsolation::Unknown => {
                return Err(invalid("this node does not know the saved isolation policy"));
            }
        }
        Ok(*project)
    }

    pub(super) async fn run_isolated(
        &self,
        job: &Job,
        principal: &Principal,
        spec: SessionSpec,
        after: AfterRun,
        isolation: WorkspaceIsolation,
        notice: String,
    ) -> Made {
        match self.start_isolated(job, principal, spec, after, isolation, notice).await {
            Ok(session) => Made::outcome(RunOutcome::Started { session }),
            Err((error, binding)) => {
                if let Some(binding) = binding
                    && let Err(settle) = self.cleanup_isolation(binding).await
                {
                    tracing::warn!(workspace = %binding.workspace, %settle, "isolated run cleanup remains unsettled");
                }
                Made::outcome(failed(&error, None))
            }
        }
    }

    async fn start_isolated(
        &self,
        job: &Job,
        principal: &Principal,
        spec: SessionSpec,
        after: AfterRun,
        isolation: WorkspaceIsolation,
        notice: String,
    ) -> Result<SessionId, (WireError, Option<ScheduleRunBinding>)> {
        let prepared = self.prepare_isolated(principal, &spec, isolation).await.map_err(|error| (error, None))?;
        let (admission, intent) = prepared;
        let store = self.sessions.store();
        let binding = ScheduleRunBinding {
            schedule: job.schedule,
            run: job.run.number,
            workspace: WorkspaceId::random(),
            session: SessionId::random(),
            after,
            stage: RunBindingStage::Bound,
        };
        store.bind_schedule_run(binding).await
            .map_err(|error| (WireError::new(error.code(), error.to_string()), None))?;
        let started = async {
            let workplace = self.workspaces.provision_workspace(binding.workspace, admission, intent, creator(principal)).await
                .map_err(|error| error.wire())?;
            self.launch_isolated(principal, spec, &workplace, binding.session, notice).await?;
            let _accepted = self.sessions.prompt(
                binding.session,
                vec![ContentBlock::Text { text: job.definition.prompt.as_str().to_owned() }],
                Delivery::Queue,
                creator(principal),
            ).await.map_err(|error| wire(&error))?;
            store.advance_schedule_run_binding(binding, RunBindingStage::Started).await
                .map_err(|error| WireError::new(error.code(), error.to_string()))?;
            Ok(binding.session)
        }.await;
        started.map_err(|error| (error, Some(binding)))
    }

    async fn prepare_isolated(
        &self,
        principal: &Principal,
        spec: &SessionSpec,
        isolation: WorkspaceIsolation,
    ) -> Result<(std::sync::Arc<crate::workspace::ProjectAdmission>, WorkspaceProvision), WireError> {
        let project = self.isolation_project(principal, spec, &isolation).await?;
        self.check_new_spec(principal, spec).await?;
        let admission = self.workspaces.admit_project(project).await.map_err(|error| error.wire())?;
        let intent = WorkspaceProvision {
            project,
            isolation,
            automation: WorkspaceAutomation::Allowed,
            title: spec.title.clone(),
            visibility: WorkspaceVisibility::Foreground,
        };
        Ok((admission, intent))
    }

    async fn launch_isolated(
        &self,
        principal: &Principal,
        mut spec: SessionSpec,
        workplace: &Provisioned,
        session: SessionId,
        notice: String,
    ) -> Result<(), WireError> {
        let store = self.sessions.store();
        spec.place = Some(Box::new(workplace.place.clone()));
        self.sessions.inherit_listing(&mut spec, self.local_creator(principal)).await.map_err(|error| wire(&error))?;
        let lease = self.workspaces.admit_workspace_start(workplace, WorkspaceResource::Session { session })
            .await.map_err(|error| error.wire())?;
        let created = self.create_starting(principal, spec, (None, Some(notice)), None, None, Some(&lease)).await;
        let settled = if created.is_ok() {
            store.complete_registry_start(lease).await
        } else {
            // A failed durable create may still be winding down. Do not
            // permit backing removal until the exact actor has stopped.
            if let Some(handle) = self.sessions.live(session) {
                let _ended = handle.ended().await;
            }
            store.fail_registry_start(lease).await
        };
        settled.map_err(|error| WireError::new(error.code(), error.to_string()))?;
        created.map(|_| ())
    }
}
