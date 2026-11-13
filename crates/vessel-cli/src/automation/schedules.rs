//! `vessel schedule …`: the arguments made into a `ScheduleDefinition`,
//! and schedules and runs printed one per line.

use std::fmt::Write as _;

use vessel_client::NodeClient;
use vessel_proto::automation::{
    AfterRun, Cadence, CatchUp, EverySeconds, MaxRuns, RunOutcome, RunTrigger, ScheduleDefinition,
    ScheduleInfo, ScheduleRun, ScheduleState, ScheduleTarget, TimeZoneName,
};
use vessel_proto::{SessionMode, SessionSpec};

use super::{find_schedule, first_line, out, parse, short, text_of};
use crate::args::{ScheduleArgs, ScheduleCommand, Target};
use crate::connect::connect;
use crate::error::CliError;
use crate::selector::SessionSelector;
use crate::workspace;

pub(crate) async fn schedule(target: &Target, command: ScheduleCommand) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    match command {
        ScheduleCommand::List => {
            let text: String = client.schedules().await?.iter().map(line).collect();
            out(&text).await
        }
        ScheduleCommand::Show { schedule } => {
            let info = find_schedule(&client, &schedule).await?;
            out(&details(&info)).await
        }
        ScheduleCommand::Runs { schedule } => {
            let info = find_schedule(&client, &schedule).await?;
            let runs = client.schedule_runs(info.id).await?;
            let text: String = runs.iter().map(run_line).collect();
            out(&text).await
        }
        ScheduleCommand::Create(args) => {
            let paused = args.paused;
            let definition = definition(&client, *args, None).await?;
            let made = client.create_schedule(definition, paused).await?;
            out(&line(&made)).await
        }
        ScheduleCommand::Edit { schedule, change } => {
            let info = find_schedule(&client, &schedule).await?;
            if change.paused {
                return Err(CliError::Usage(
                    "`--paused` is for `create`; use `vessel schedule pause`",
                ));
            }
            let definition = definition(&client, *change, Some(info.definition)).await?;
            let changed = client
                .update_schedule(info.id, info.revision, definition)
                .await?;
            out(&line(&changed)).await
        }
        ScheduleCommand::Pause { schedule } => {
            let info = find_schedule(&client, &schedule).await?;
            out(&line(&client.pause_schedule(info.id).await?)).await
        }
        ScheduleCommand::Resume { schedule } => {
            let info = find_schedule(&client, &schedule).await?;
            out(&line(&client.resume_schedule(info.id).await?)).await
        }
        ScheduleCommand::Run { schedule } => {
            let info = find_schedule(&client, &schedule).await?;
            out(&run_line(&client.run_schedule(info.id).await?)).await
        }
        ScheduleCommand::Delete { schedule } => {
            let info = find_schedule(&client, &schedule).await?;
            client.delete_schedule(info.id).await?;
            out(&format!("deleted {}\n", info.id)).await
        }
    }
}

/// The definition the arguments say: a new one, or `old` with what they
/// change.
async fn definition(
    client: &NodeClient,
    args: ScheduleArgs,
    old: Option<ScheduleDefinition>,
) -> Result<ScheduleDefinition, CliError> {
    let cadence = cadence(&args)?;
    let target = target(client, &args, old.as_ref().map(|old| &old.target)).await?;
    let prompt = text_of(args.prompt, args.prompt_file.as_deref())
        .await?
        .map(|text| parse(&text, "prompt"))
        .transpose()?;
    let name = args.name.map(|name| parse(&name, "name")).transpose()?;
    let max_runs = args
        .max_runs
        .map(|runs| {
            MaxRuns::try_from(runs)
                .map_err(|error| CliError::Automation(format!("--max-runs: {error}")))
        })
        .transpose()?;
    let catch_up = args.catch_up.as_deref().map(catch_up).transpose()?;
    let Some(old) = old else {
        return Ok(ScheduleDefinition {
            name,
            cadence: cadence.ok_or(CliError::Usage("give `--every` or `--cron`"))?,
            target: target.ok_or(CliError::Usage("give `--session` or `--harness`"))?,
            prompt: prompt.ok_or(CliError::Usage("give `--prompt` or `--prompt-file`"))?,
            catch_up: catch_up.unwrap_or(CatchUp::Once),
            max_runs,
            ends_at: args.ends_at,
        });
    };
    Ok(ScheduleDefinition {
        name: name.or(old.name),
        cadence: cadence.unwrap_or(old.cadence),
        target: target.unwrap_or(old.target),
        prompt: prompt.unwrap_or(old.prompt),
        catch_up: catch_up.unwrap_or(old.catch_up),
        max_runs: max_runs.or(old.max_runs),
        ends_at: args.ends_at.or(old.ends_at),
    })
}

fn cadence(args: &ScheduleArgs) -> Result<Option<Cadence>, CliError> {
    if let Some(every) = &args.every {
        let seconds = EverySeconds::try_from(seconds(every)?)
            .map_err(|error| CliError::Automation(format!("--every: {error}")))?;
        return Ok(Some(Cadence::Every {
            seconds,
            anchor: None,
        }));
    }
    let Some(cron) = &args.cron else {
        return Ok(None);
    };
    let zone = match &args.tz {
        Some(zone) => parse(zone, "--tz")?,
        None => this_zone()?,
    };
    Ok(Some(Cadence::Cron {
        expr: parse(cron, "--cron")?,
        zone,
    }))
}

/// The time zone this machine is in, by its IANA name.
fn this_zone() -> Result<TimeZoneName, CliError> {
    let zone = jiff::tz::TimeZone::system();
    let name = zone.iana_name().ok_or(CliError::Usage(
        "this machine's time zone has no IANA name; give `--tz` (such as Europe/London)",
    ))?;
    parse(name, "this machine's time zone")
}

/// `90s`, `30m`, `2h` or `1d` in seconds.
fn seconds(text: &str) -> Result<u32, CliError> {
    let invalid =
        || CliError::Automation(format!("`{text}` is not a length like `30m`, `2h` or `1d`"));
    let trimmed = text.trim();
    let (number, scale) = match trimmed.char_indices().last() {
        Some((at, 's')) => (trimmed.get(..at), 1),
        Some((at, 'm')) => (trimmed.get(..at), 60),
        Some((at, 'h')) => (trimmed.get(..at), 3_600),
        Some((at, 'd')) => (trimmed.get(..at), 86_400),
        Some(_) | None => (Some(trimmed), 1),
    };
    number
        .ok_or_else(invalid)?
        .parse::<u32>()
        .ok()
        .and_then(|number| number.checked_mul(scale))
        .ok_or_else(invalid)
}

fn catch_up(text: &str) -> Result<CatchUp, CliError> {
    match text {
        "once" => Ok(CatchUp::Once),
        "skip" => Ok(CatchUp::Skip),
        _ => Err(CliError::Usage("`--catch-up` is `once` or `skip`")),
    }
}

/// The target the arguments name, if any. Editing a schedule of new
/// sessions (`old`), what the arguments do not say of its spec stays as it
/// was: its place, profile, model, options and title are not dropped
/// because one of them changed (the join's code review, finding 10).
async fn target(
    client: &NodeClient,
    args: &ScheduleArgs,
    old: Option<&ScheduleTarget>,
) -> Result<Option<ScheduleTarget>, CliError> {
    if let Some(session) = &args.session {
        let selector: SessionSelector = parse(session, "--session")?;
        let info = selector.resolve(client).await?;
        return Ok(Some(ScheduleTarget::Session { session: info.id }));
    }
    let changes = args.harness.is_some()
        || args.profile.is_some()
        || args.project.is_some()
        || args.worktree.is_some()
        || args.model.is_some()
        || args.keep;
    if !changes {
        return Ok(None);
    }
    let (mut spec, mut after) = match (old, &args.harness) {
        (Some(ScheduleTarget::NewSession { spec, after }), harness) => {
            let mut spec = (**spec).clone();
            if let Some(harness) = harness {
                spec.harness = harness.clone();
            }
            (spec, *after)
        }
        (_, Some(harness)) => {
            let mode = SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            };
            (SessionSpec::new(harness.clone(), mode), AfterRun::Archive)
        }
        (_, None) => {
            return Err(CliError::Usage(
                "`--profile`, `--project`, `--worktree`, `--model` and `--keep` go with \
                 `--harness`, or edit a schedule that starts new sessions",
            ));
        }
    };
    if let Some(model) = &args.model
        && let SessionMode::Structured { model: wanted, .. } = &mut spec.mode
    {
        *wanted = Some(vessel_proto::ModelId::new(model.clone()));
    }
    if let Some(profile) = &args.profile {
        spec = spec.with_profile(profile.clone());
    }
    let place = workspace::place(
        client,
        args.project.as_deref(),
        None,
        false,
        args.worktree.as_deref(),
    )
    .await?;
    if let Some(place) = place {
        spec.cwd = None;
        spec = spec.with_place(place);
    }
    if args.keep {
        after = AfterRun::Keep;
    }
    Ok(Some(ScheduleTarget::NewSession {
        spec: Box::new(spec),
        after,
    }))
}

/// One schedule on a line: id, name, when, state, runs.
fn line(info: &ScheduleInfo) -> String {
    let name = info
        .definition
        .name
        .as_ref()
        .map_or("-", |name| name.as_str());
    format!(
        "{}  {name}  {}  {}  runs {}\n",
        short(&info.id),
        cadence_text(&info.definition.cadence),
        state_text(&info.state),
        info.runs
    )
}

fn details(info: &ScheduleInfo) -> String {
    let mut text = line(info);
    let definition = &info.definition;
    let target = match &definition.target {
        ScheduleTarget::NewSession { spec, after } => format!(
            "a new {} session each run ({})",
            spec.harness,
            match after {
                AfterRun::Archive => "archived after its first turn",
                AfterRun::Keep => "kept",
                AfterRun::Unknown => "?",
            }
        ),
        ScheduleTarget::Session { session } => format!("session {session}, through its queue"),
        ScheduleTarget::Unknown => "a target this client does not know".to_owned(),
    };
    let _infallible = writeln!(text, "id: {}", info.id);
    let _infallible = writeln!(text, "target: {target}");
    let _infallible = writeln!(text, "acts as: {:?}", info.defined_by);
    if let Some(max) = definition.max_runs {
        let _infallible = writeln!(text, "at most {} runs", max.get());
    }
    if let Some(ends) = definition.ends_at {
        let _infallible = writeln!(text, "ends at {ends}");
    }
    if let Some(run) = &info.last_run {
        let _infallible = write!(text, "last run: {}", run_line(run));
    }
    let _infallible = writeln!(text, "prompt:\n{}", definition.prompt.as_str());
    text
}

fn cadence_text(cadence: &Cadence) -> String {
    match cadence {
        Cadence::Every { seconds, .. } => format!("every {}s", seconds.get()),
        Cadence::Cron { expr, zone } => format!("{} ({})", expr.as_str(), zone.as_str()),
        Cadence::Unknown => "?".to_owned(),
    }
}

fn state_text(state: &ScheduleState) -> String {
    match state {
        ScheduleState::Active { next_due } => format!("next {next_due}"),
        ScheduleState::Paused { since } => format!("paused since {since}"),
        ScheduleState::Ended { at, reason } => format!("ended {at} ({reason:?})"),
        ScheduleState::Unknown => "?".to_owned(),
    }
}

/// One run on a line: its number, when it was due, why, what came of it.
fn run_line(run: &ScheduleRun) -> String {
    let trigger = match &run.trigger {
        RunTrigger::Due => "due".to_owned(),
        RunTrigger::CaughtUp { missed } => format!("caught up ({missed} missed)"),
        RunTrigger::Asked { by } => format!("asked by {by:?}"),
        RunTrigger::Unknown => "?".to_owned(),
    };
    let outcome = match &run.outcome {
        RunOutcome::Making => "being made".to_owned(),
        RunOutcome::Started { session } => format!("started {session}"),
        RunOutcome::Waiting { session } => format!("{session} waits for an answer"),
        RunOutcome::Finished { session, end } => format!("{session} finished ({end:?})"),
        RunOutcome::Prompted { session, delivered } => {
            format!("prompted {session} ({delivered:?})")
        }
        RunOutcome::Skipped { reason } => format!("skipped ({reason:?})"),
        RunOutcome::Failed { error, .. } => {
            format!("failed: {}", first_line(&error.message, 120))
        }
        RunOutcome::Interrupted => "interrupted by the node's stop".to_owned(),
        RunOutcome::Unknown => "?".to_owned(),
    };
    format!(
        "#{}  {}  {trigger}  {outcome}\n",
        run.number.get(),
        run.due_at
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_read_in_their_unit() {
        assert_eq!(seconds("90s").unwrap(), 90);
        assert_eq!(seconds("30m").unwrap(), 1_800);
        assert_eq!(seconds("2h").unwrap(), 7_200);
        assert_eq!(seconds("1d").unwrap(), 86_400);
        assert_eq!(seconds("120").unwrap(), 120);
        assert!(seconds("soon").is_err());
        assert!(seconds("99999999d").is_err());
    }
}
