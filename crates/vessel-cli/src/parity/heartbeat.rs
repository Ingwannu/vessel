//! A heartbeat is an existing session-targeted schedule, never a second scheduler.
use super::schedule_output;
use crate::{
    args::{
        Target,
        parity::{HeartbeatCommand, Presentation, ScheduleSelector},
    },
    connect::connect,
    error::CliError,
};
use vessel_proto::{
    SessionId,
    automation::{Cadence, CatchUp, MaxRuns, ScheduleDefinition, ScheduleInfo, ScheduleTarget},
};

pub(super) async fn run(
    target: &Target,
    presentation: Presentation,
    command: HeartbeatCommand,
) -> Result<(), CliError> {
    let caller = caller()?;
    let client = connect(target).await?.client;
    match command {
        HeartbeatCommand::Create {
            prompt,
            cron,
            timezone,
            name,
            max_runs,
            expires_in,
        } => {
            let ends_at = expires_in
                .map(|lifetime| {
                    let delta =
                        chrono::TimeDelta::from_std(lifetime.duration()).map_err(|_range| {
                            CliError::Usage("heartbeat expiry exceeds supported time")
                        })?;
                    chrono::Utc::now()
                        .checked_add_signed(delta)
                        .ok_or(CliError::Usage("heartbeat expiry exceeds supported time"))
                })
                .transpose()?;
            let definition = ScheduleDefinition {
                name,
                cadence: Cadence::Cron {
                    expr: cron,
                    zone: timezone,
                },
                target: ScheduleTarget::Session { session: caller },
                prompt,
                catch_up: CatchUp::Once,
                max_runs: max_runs
                    .map(|runs| {
                        MaxRuns::try_from(runs.get())
                            .map_err(|_range| CliError::Usage("invalid heartbeat run limit"))
                    })
                    .transpose()?,
                ends_at,
            };
            let schedule = client.create_schedule(definition, false).await?;
            schedule_output::one(presentation, &schedule).await
        }
        HeartbeatCommand::Update { id, cron, timezone } => {
            let old = owned(&client, &id, caller).await?;
            let mut definition = old.definition;
            definition.cadence = Cadence::Cron {
                expr: cron,
                zone: timezone,
            };
            let changed = client
                .update_schedule(old.id, old.revision, definition)
                .await?;
            schedule_output::one(presentation, &changed).await
        }
        HeartbeatCommand::Delete { id } => {
            let old = owned(&client, &id, caller).await?;
            client.delete_schedule(old.id).await?;
            schedule_output::deleted(presentation, old.id).await
        }
    }
}
fn caller() -> Result<SessionId, CliError> {
    let text = std::env::var("VESSEL_AGENT_SESSION").map_err(|_absent| {
        CliError::Usage("heartbeat requires VESSEL_AGENT_SESSION from the calling agent")
    })?;
    text.parse()
        .map_err(|_invalid| CliError::Usage("the caller session identity is invalid"))
}
async fn owned(
    client: &vessel_client::NodeClient,
    id: &ScheduleSelector,
    caller: SessionId,
) -> Result<ScheduleInfo, CliError> {
    let schedule = crate::automation::find_schedule(client, id.text()).await?;
    match schedule.definition.target {
        ScheduleTarget::Session { session } if session == caller => Ok(schedule),
        ScheduleTarget::Session { .. }
        | ScheduleTarget::NewSession { .. }
        | ScheduleTarget::Unknown => Err(CliError::Usage(
            "the selected schedule is not this caller's heartbeat",
        )),
    }
}
