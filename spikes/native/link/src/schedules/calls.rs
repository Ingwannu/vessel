//! Bounded requests through the current link actor. A run-now operation must
//! leave fleet, session and input traffic free to keep moving.
use super::{Answer, Value, interrupted};
use crate::{
    Event, State,
    connections::{RequestFailure, SelectedConnection},
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use vessel_app_view::schedules::{
    load::{Failure, MAX_IN_FLIGHT},
    page::{Ask, Request, Snapshot, WriteValue},
};
use vessel_client::CallError;
use vessel_proto::{
    automation::{
        DeleteSchedule, ListSchedules, PauseSchedule, ResumeSchedule, RunSchedule, ScheduleTarget,
    },
    workspace::{ListProjects, ListWorktrees, SessionPlace},
};

mod form;

const LIST_DEADLINE: Duration = Duration::from_secs(15);
const FACT_DEADLINE: Duration = Duration::from_secs(2);
const RUN_DEADLINE: Duration = Duration::from_secs(120);
const WRITE_DEADLINE: Duration = Duration::from_secs(20);

#[derive(Debug)]
pub(crate) struct Hub(Arc<Semaphore>);

impl Default for Hub {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(MAX_IN_FLIGHT)))
    }
}

impl State {
    pub(crate) async fn schedule(&self, request: Request) {
        let Ok(selected) = self.selected_connection(request.connection()) else {
            self.emit(Event::Schedules(Answer::refused(
                request,
                Failure::ConnectionChanged,
            )))
            .await;
            return;
        };
        let Ok(permit) = Arc::clone(&self.schedule_calls.0).try_acquire_owned() else {
            self.emit(Event::Schedules(Answer::refused(request, Failure::Busy)))
                .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let guard = selected.delivery_guard();
            // Boxed, as the list below: schedule definitions (now with their
            // isolation) put these futures past clippy's large_futures limit.
            let result = Box::pin(execute(&selected, request.ask)).await;
            // Even after the connection goes away, one terminal event retires
            // the ticket. A weak guard rechecks validity at the receiver.
            drop(selected);
            let slot = events.reserve().await;
            drop(permit);
            if let Ok(slot) = slot {
                slot.send(Event::Schedules(Answer {
                    request,
                    result,
                    guard: Some(guard),
                }));
            }
        }));
    }
}

async fn execute(client: &SelectedConnection, ask: Ask) -> Result<Value, Failure> {
    let deadline = match ask {
        Ask::List => {
            return Box::pin(list(client))
                .await
                .map(|snapshot| Value::Listed(Box::new(snapshot)));
        }
        Ask::Run(_) => RUN_DEADLINE,
        Ask::Pause(_) | Ask::Resume(_) | Ask::Delete(_) => WRITE_DEADLINE,
    };
    tokio::time::timeout(deadline, write(client, ask))
        .await
        .unwrap_or(Err(Failure::Unconfirmed))
}

async fn write(client: &SelectedConnection, ask: Ask) -> Result<Value, Failure> {
    let result = match ask {
        Ask::List => return Err(Failure::Unexpected),
        Ask::Pause(schedule) => client
            .call(PauseSchedule { schedule })
            .await
            .map(|info| WriteValue::Schedule(Box::new(info))),
        Ask::Resume(schedule) => client
            .call(ResumeSchedule { schedule })
            .await
            .map(|info| WriteValue::Schedule(Box::new(info))),
        Ask::Run(schedule) => client
            .call(RunSchedule { schedule })
            .await
            .map(WriteValue::Run),
        Ask::Delete(schedule) => client
            .call(DeleteSchedule { schedule })
            .await
            .map(|_receipt| WriteValue::Deleted),
    };
    result
        .map(Value::Written)
        .map_err(|error| failure(ask, error))
}

async fn list(client: &SelectedConnection) -> Result<Snapshot, Failure> {
    let schedules = timed_read(LIST_DEADLINE, client.call(ListSchedules {}))
        .await?
        .schedules;
    let (projects, worktrees, scopes) = tokio::join!(
        projects(client, &schedules),
        worktrees(client, &schedules),
        timed_read(FACT_DEADLINE, client.call(vessel_proto::GetDeviceSelf {})),
    );
    Ok(Snapshot {
        schedules,
        scopes: scopes.map(|report| report.scopes),
        read_at: chrono::Utc::now(),
        projects,
        worktrees,
    })
}

async fn projects(
    client: &SelectedConnection,
    list: &[vessel_proto::automation::ScheduleInfo],
) -> Result<Vec<vessel_proto::workspace::ProjectInfo>, Failure> {
    if list.iter().any(|row| match &row.definition.target {
        ScheduleTarget::NewSession { spec, .. } => match spec.place.as_deref() {
            Some(SessionPlace::Project { .. }) => true,
            Some(SessionPlace::Worktree { .. }) => false,
            None => spec.cwd.is_some(),
        },
        ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => false,
    }) {
        timed_read(FACT_DEADLINE, client.call(ListProjects {}))
            .await
            .map(|list| list.projects)
    } else {
        Ok(Vec::new())
    }
}

async fn worktrees(
    client: &SelectedConnection,
    list: &[vessel_proto::automation::ScheduleInfo],
) -> Result<Vec<vessel_proto::workspace::WorktreeInfo>, Failure> {
    if list.iter().any(|row| match &row.definition.target {
        ScheduleTarget::NewSession { spec, .. } => {
            matches!(spec.place.as_deref(), Some(SessionPlace::Worktree { .. }))
        }
        ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => false,
    }) {
        timed_read(FACT_DEADLINE, client.call(ListWorktrees { project: None }))
            .await
            .map(|list| list.worktrees)
    } else {
        Ok(Vec::new())
    }
}

async fn timed_read<T>(
    deadline: Duration,
    read: impl Future<Output = Result<T, RequestFailure>>,
) -> Result<T, Failure> {
    tokio::time::timeout(deadline, read)
        .await
        .map_err(|_deadline| Failure::TimedOut)?
        .map_err(|error| failure(Ask::List, error))
}

fn failure(ask: Ask, error: RequestFailure) -> Failure {
    match error {
        RequestFailure::Refused(why) => interrupted(
            ask,
            match why {
                vessel_app_view::connection::Refusal::Changed => Failure::ConnectionChanged,
                vessel_app_view::connection::Refusal::Closed => Failure::Offline,
            },
        ),
        RequestFailure::Call(error) => match error {
            CallError::Remote(wire) => Failure::Node(wire.code),
            CallError::NotInProtocol { .. } => Failure::Older,
            CallError::UnassignedProtocol { .. } => Failure::NotEnabled,
            CallError::RequestIdsExhausted => Failure::Exhausted,
            CallError::Disconnected => interrupted(ask, Failure::Offline),
            CallError::UnexpectedReply(_) => interrupted(ask, Failure::Unexpected),
        },
    }
}

#[cfg(test)]
mod tests;
