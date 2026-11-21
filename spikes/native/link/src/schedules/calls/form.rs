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
    GetDeviceSelf, GetNode, ListHarnesses,
    agents::ListAgents,
    automation::TimeZoneName,
    workspace::{ListProjects, ListWorktrees},
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
                Ask::Targets | Ask::Options(_) | Ask::Preview(_) => LIST_DEADLINE,
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
        Ask::Targets => targets(client)
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
            };
            result
                .map(|value| Value::Saved(Box::new(value)))
                .map_err(|error| form_failure(ask, error))
        }
    }
}

async fn targets(client: &SelectedConnection) -> Result<Targets, Failure> {
    // The lookup may touch OS files. Keep it off the actor and the async
    // worker pool. Absence is handled by the node zone or the form field.
    let zone = tokio::task::spawn_blocking(device_zone);
    let (machine, projects, worktrees, harnesses, agents, scopes, device_zone) = tokio::join!(
        client.call(GetNode {}),
        client.call(ListProjects {}),
        client.call(ListWorktrees { project: None }),
        client.call(ListHarnesses {}),
        client.call(ListAgents {}),
        client.call(GetDeviceSelf {}),
        zone,
    );
    let read = |error| form_failure(&Ask::Targets, error);
    Ok(Targets {
        machine_zone: machine.map_err(read)?.timezone,
        device_zone: device_zone.ok().flatten(),
        projects: projects.map_err(read)?.projects,
        worktrees: worktrees.map_err(read)?.worktrees,
        harnesses: harnesses.map_err(read)?,
        agents: agents.map_err(read)?,
        scopes: scopes.map(|report| report.scopes).map_err(read),
    })
}

fn device_zone() -> Option<TimeZoneName> {
    iana_time_zone::get_timezone().ok()?.parse().ok()
}

fn form_failure(ask: &Ask, error: RequestFailure) -> Failure {
    let read = failure(page::Ask::List, error);
    if matches!(read, Failure::ConnectionChanged | Failure::Offline | Failure::Unexpected) {
        ask.interrupted(read)
    } else {
        read
    }
}
