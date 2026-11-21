//! Detached editor calls share the schedule request capacity and delivery fence.
use super::{LIST_DEADLINE, WRITE_DEADLINE, failure};
use crate::{
    Event, State,
    connections::{RequestFailure, SelectedConnection},
    schedules::form::Answer,
};
use std::sync::Arc;
use vessel_app_view::schedules::{
    editor::{
        Save,
        io::{Ask, Request, Targets, Value},
    },
    load::Failure,
    page,
};
use vessel_proto::{
    GetDeviceSelf, GetNode, Harness, ListHarnesses,
    agents::{AgentInfo, ListAgents},
    automation::TimeZoneName,
    workspace::{ListProjects, ListWorktrees, ProjectInfo, WorktreeInfo},
};

impl State {
    pub(crate) async fn schedule_form(&self, request: Request) {
        let Ok(selected) = self.selected_connection(&request.connection) else {
            self.emit(Event::ScheduleForm(Answer::refused(
                request,
                Failure::ConnectionChanged,
            )))
            .await;
            return;
        };
        let Ok(permit) = Arc::clone(&self.schedule_calls.0).try_acquire_owned() else {
            self.emit(Event::ScheduleForm(Answer::refused(request, Failure::Busy)))
                .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let guard = selected.delivery_guard();
            let limit = match request.ask {
                Ask::Save(_) => WRITE_DEADLINE,
                Ask::Targets | Ask::HeartbeatFacts | Ask::Options(_) | Ask::Preview(_) => {
                    LIST_DEADLINE
                }
            };
            let result = tokio::time::timeout(limit, execute(&selected, &request.ask))
                .await
                .unwrap_or_else(|_| Err(request.ask.interrupted(Failure::TimedOut)));
            drop(selected);
            let slot = events.reserve().await;
            drop(permit);
            if let Ok(slot) = slot {
                slot.send(Event::ScheduleForm(Answer {
                    request,
                    result,
                    guard: Some(guard),
                }));
            }
        }));
    }
}

async fn execute(client: &SelectedConnection, ask: &Ask) -> Result<Value, Failure> {
    match ask {
        Ask::Targets | Ask::HeartbeatFacts => targets(client, ask)
            .await
            .map(|value| Value::Targets(Box::new(value))),
        Ask::Options(options) => client
            .call(options.clone())
            .await
            .map(|value| Value::Options(Box::new(value)))
            .map_err(|error| form_failure(ask, error)),
        Ask::Preview(preview) => client
            .call(preview.clone())
            .await
            .map(|value| Value::Preview(Box::new(value)))
            .map_err(|error| form_failure(ask, error)),
        Ask::Save(save) => {
            let result = match save.as_ref() {
                Save::Create(create) => client.call(create.clone()).await,
                Save::Update(update) => client.call(update.clone()).await,
                Save::Heartbeat(update) => client.call(update.clone()).await,
            };
            result
                .map(|value| Value::Saved(Box::new(value)))
                .map_err(|error| form_failure(ask, error))
        }
    }
}

async fn targets(client: &SelectedConnection, ask: &Ask) -> Result<Targets, Failure> {
    // The lookup may touch OS files. Keep it off the actor and the async
    // worker pool. Absence is handled by the node zone or the form field.
    let zone = tokio::task::spawn_blocking(device_zone);
    let (machine, scopes, device_zone, choices) = tokio::join!(
        client.call(GetNode {}),
        client.call(GetDeviceSelf {}),
        zone,
        choices(client, ask),
    );
    let read = |error| form_failure(ask, error);
    let machine_zone = machine.map_err(read)?.timezone;
    let choices = choices?;
    Ok(Targets {
        machine_zone,
        device_zone: device_zone.ok().flatten(),
        projects: choices.projects,
        worktrees: choices.worktrees,
        harnesses: choices.harnesses,
        agents: choices.agents,
        scopes: scopes.map(|report| report.scopes).map_err(read),
    })
}

#[derive(Default)]
struct Choices {
    projects: Vec<ProjectInfo>,
    worktrees: Vec<WorktreeInfo>,
    harnesses: Vec<Harness>,
    agents: Vec<AgentInfo>,
}

async fn choices(client: &SelectedConnection, ask: &Ask) -> Result<Choices, Failure> {
    if matches!(ask, Ask::HeartbeatFacts) {
        return Ok(Choices::default());
    }
    let (projects, worktrees, harnesses, agents) = tokio::join!(
        client.call(ListProjects {}),
        client.call(ListWorktrees { project: None }),
        client.call(ListHarnesses {}),
        client.call(ListAgents {}),
    );
    let read = |error| form_failure(ask, error);
    Ok(Choices {
        projects: projects.map_err(read)?.projects,
        worktrees: worktrees.map_err(read)?.worktrees,
        harnesses: harnesses.map_err(read)?.harnesses,
        agents: agents.map_err(read)?.agents,
    })
}

fn device_zone() -> Option<TimeZoneName> {
    iana_time_zone::get_timezone().ok()?.parse().ok()
}

fn form_failure(ask: &Ask, error: RequestFailure) -> Failure {
    let read = failure(page::Ask::List, error);
    if matches!(
        read,
        Failure::ConnectionChanged | Failure::Offline | Failure::Unexpected
    ) {
        ask.interrupted(read)
    } else {
        read
    }
}
