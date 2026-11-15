//! A permit covers the call and its bounded event delivery. It is not released
//! early when a page invalidates its own ticket or stops drawing the row.
use super::{Answer, ListValue, Message, ScheduleError, Value};
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc};
use vessel_app_view::schedules::{
    load::MAX_IN_FLIGHT,
    page::{Ask, Request, WriteValue},
};
use vessel_client::NodeClient;
use vessel_proto::{
    automation::ScheduleTarget,
    workspace::{ListProjects, ListWorktrees, SessionPlace},
};

#[derive(Debug, Clone)]
pub(crate) struct Hub(Arc<Semaphore>);
impl Default for Hub {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(MAX_IN_FLIGHT)))
    }
}
impl Hub {
    pub(crate) async fn ask(
        &self,
        client: Option<NodeClient>,
        request: Request,
        events: mpsc::Sender<crate::Event>,
    ) {
        let Ok(permit) = Arc::clone(&self.0).try_acquire_owned() else {
            let _closed = events
                .send(crate::Event::Schedules(Message::Replied(Answer {
                    request,
                    result: Err(ScheduleError::Busy),
                })))
                .await;
            return;
        };
        drop(tokio::spawn(async move {
            let result = match client {
                Some(client) => run(&client, request.ask).await,
                None => Err(ScheduleError::Offline),
            };
            let _closed = events
                .send(crate::Event::Schedules(Message::Replied(Answer {
                    request,
                    result,
                })))
                .await;
            drop(permit);
        }));
    }
}
async fn list(client: &NodeClient) -> Result<Value, ScheduleError> {
    let schedules = client.schedules().await?;
    let needs_projects = schedules.iter().any(|info| match &info.definition.target {
        ScheduleTarget::NewSession { spec, .. } => match spec.place.as_deref() {
            Some(SessionPlace::Project { .. }) => true,
            Some(SessionPlace::Worktree { .. }) => false,
            None => spec.cwd.is_some(),
        },
        ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => false,
    });
    let needs_worktrees = schedules.iter().any(|info| match &info.definition.target {
        ScheduleTarget::NewSession { spec, .. } => {
            matches!(spec.place.as_deref(), Some(SessionPlace::Worktree { .. }))
        }
        ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => false,
    });
    let projects = if needs_projects {
        client
            .call(ListProjects {})
            .await
            .map(|list| list.projects)
            .map_err(ScheduleError::Call)
    } else {
        Ok(Vec::new())
    };
    let worktrees = if needs_worktrees {
        client
            .call(ListWorktrees { project: None })
            .await
            .map(|list| list.worktrees)
            .map_err(ScheduleError::Call)
    } else {
        Ok(Vec::new())
    };
    Ok(Value::List(Box::new(ListValue {
        schedules,
        projects,
        worktrees,
        read_at: chrono::Utc::now(),
    })))
}
async fn run(client: &NodeClient, ask: Ask) -> Result<Value, ScheduleError> {
    let written = match ask {
        Ask::List => return list(client).await,
        Ask::Pause(id) => WriteValue::Schedule(Box::new(client.pause_schedule(id).await?)),
        Ask::Resume(id) => WriteValue::Schedule(Box::new(client.resume_schedule(id).await?)),
        Ask::Run(id) => WriteValue::Run(client.run_schedule(id).await?),
        Ask::Delete(id) => {
            client.delete_schedule(id).await?;
            WriteValue::Deleted
        }
    };
    Ok(Value::Written(written))
}

impl crate::State {
    pub(crate) async fn schedule(&self, request: Request) {
        let client = self
            .clients
            .get(request.machine.as_str())
            .map(|connected| connected.client.clone());
        self.schedule_calls
            .ask(client, request, self.events.clone())
            .await;
    }
}
