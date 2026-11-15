//! A permit covers the call and its bounded event delivery. It is not released
//! early when a page invalidates its own ticket or stops drawing the row.
use super::{
    Answer, FormAnswer, HistoryAnswer, ListValue, Message, Request, ScheduleError, SessionAnswer,
    Value,
};
use crate::connections::SelectedConnection;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Semaphore, mpsc};
use vessel_app_view::schedules::{
    load::MAX_IN_FLIGHT,
    page::{Ask, WriteValue},
};
const REQUEST_DEADLINE: Duration = Duration::from_secs(30);
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
        client: Result<SelectedConnection, ScheduleError>,
        request: impl Into<Request>,
        events: mpsc::Sender<crate::Event>,
    ) {
        let request = request.into();
        let Ok(permit) = Arc::clone(&self.0).try_acquire_owned() else {
            let _closed = events
                .send(crate::Event::Schedules(
                    request.refused(ScheduleError::Busy),
                ))
                .await;
            return;
        };
        drop(tokio::spawn(async move {
            let message = match client {
                Ok(client) => tokio::time::timeout(REQUEST_DEADLINE, execute(&client, &request))
                    .await
                    .unwrap_or_else(|_| request.clone().refused(ScheduleError::Deadline)),
                Err(error) => request.refused(error),
            };
            let _closed = events.send(crate::Event::Schedules(message)).await;
            drop(permit);
        }));
    }
}
async fn list(client: &SelectedConnection) -> Result<Value, ScheduleError> {
    let clock = super::clock::viewer_clock().await;
    let schedules = client
        .call(vessel_proto::automation::ListSchedules {})
        .await?
        .schedules;
    // This authenticated self-query needs no device-list permission. Its failure
    // must not turn an otherwise valid list into a read-only screen.
    let scopes = client
        .call(vessel_proto::GetDeviceSelf {})
        .await
        .map(|report| report.scopes)
        .map_err(ScheduleError::from);
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
            .map_err(ScheduleError::from)
    } else {
        Ok(Vec::new())
    };
    let worktrees = if needs_worktrees {
        client
            .call(ListWorktrees { project: None })
            .await
            .map(|list| list.worktrees)
            .map_err(ScheduleError::from)
    } else {
        Ok(Vec::new())
    };
    Ok(Value::List(Box::new(ListValue {
        schedules,
        clock,
        scopes,
        projects,
        worktrees,
        read_at: chrono::Utc::now(),
    })))
}
async fn run(client: &SelectedConnection, ask: Ask) -> Result<Value, ScheduleError> {
    let written = match ask {
        Ask::List => return list(client).await,
        Ask::Pause(id) => WriteValue::Schedule(Box::new(
            client
                .call(vessel_proto::automation::PauseSchedule { schedule: id })
                .await?,
        )),
        Ask::Resume(id) => WriteValue::Schedule(Box::new(
            client
                .call(vessel_proto::automation::ResumeSchedule { schedule: id })
                .await?,
        )),
        Ask::Run(id) => WriteValue::Run(
            client
                .call(vessel_proto::automation::RunSchedule { schedule: id })
                .await?,
        ),
        Ask::Delete(id) => {
            client
                .call(vessel_proto::automation::DeleteSchedule { schedule: id })
                .await?;
            WriteValue::Deleted
        }
    };
    Ok(Value::Written(written))
}

impl crate::State {
    pub(crate) async fn schedule(&self, request: impl Into<Request>) {
        let request = request.into();
        let client = self
            .selected_connection(request.connection())
            .map_err(ScheduleError::NotSelected);
        self.schedule_calls
            .ask(client, request, self.events.clone())
            .await;
    }
}

async fn execute(client: &SelectedConnection, request: &Request) -> Message {
    match request {
        Request::Row(request) => Message::Replied(Answer {
            request: request.clone(),
            result: run(client, request.ask).await,
        }),
        Request::Form(request) => Message::FormReplied(FormAnswer {
            request: request.clone(),
            result: super::form::run(client, request.operation()).await,
        }),
        Request::History(request) => Message::HistoryReplied(HistoryAnswer {
            request: request.clone(),
            result: client
                .call(request.input())
                .await
                .map_err(ScheduleError::from),
        }),
        Request::Lookup(request) => Message::SessionReplied(SessionAnswer {
            request: request.clone(),
            result: client
                .call(request.input())
                .await
                .map_err(ScheduleError::from),
        }),
    }
}
