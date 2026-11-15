//! Normalize invocation choices before connecting; cadence and expiry cannot mutate partially.
use crate::{
    args::{ScheduleArgs, ScheduleCommand, Target},
    error::CliError,
};
use vessel_proto::automation::{Cadence, CronExpr, ScheduleTarget};

pub(super) fn preflight(command: &mut ScheduleCommand, target: &Target) -> Result<(), CliError> {
    match command {
        ScheduleCommand::Create(args) => {
            if args.parity.reset.no_max_runs || args.parity.reset.no_expires_in {
                return Err(CliError::Usage(
                    "limit reset flags apply only to schedule update",
                ));
            }
            normalize(args, target, Operation::Create)
        }
        ScheduleCommand::Edit { change, .. } => {
            if change.parity.run_now {
                return Err(CliError::Usage("--run-now applies only to schedule create"));
            }
            normalize(change, target, Operation::Update)
        }
        ScheduleCommand::Preview { .. }
        | ScheduleCommand::List
        | ScheduleCommand::Show { .. }
        | ScheduleCommand::Runs { .. }
        | ScheduleCommand::Pause { .. }
        | ScheduleCommand::Resume { .. }
        | ScheduleCommand::Run { .. }
        | ScheduleCommand::Delete { .. } => Ok(()),
    }
}

#[derive(Clone, Copy)]
enum Operation {
    Create,
    Update,
}

fn normalize(
    args: &mut ScheduleArgs,
    target: &Target,
    operation: Operation,
) -> Result<(), CliError> {
    if args.parity.mode.is_some() || args.parity.thinking.is_some() {
        return Err(CliError::ParityBlocked(
            "schedule mode/thinking awaits W22's validated provider configuration contract",
        ));
    }
    if args.parity.target.is_some() {
        return Err(CliError::ParityBlocked(
            "legacy schedule --target requires a reviewed target decoder; use --provider or heartbeat",
        ));
    }
    let new_target = args.parity.provider.is_some() || args.positional_prompt.is_some();
    if let Some(provider) = args.parity.provider.take() {
        let (driver, model) = provider.into_parts();
        args.harness = Some(driver);
        if let Some(model) = model {
            if args
                .model
                .as_deref()
                .is_some_and(|explicit| explicit != model.as_str())
            {
                return Err(CliError::Usage(
                    "--model conflicts with provider/model shorthand",
                ));
            }
            args.model = Some(model.as_str().to_owned());
        }
    }
    if let Some(prompt) = args.positional_prompt.take() {
        args.prompt = Some(prompt);
    }
    if new_target && matches!(operation, Operation::Create) && args.parity.cwd.is_none() {
        let (resolved, _book) = crate::nodes::resolve(target)?;
        if target.node.is_some() || !crate::connect::is_here(&resolved) {
            return Err(CliError::Usage(
                "schedule create requires --cwd with an explicit or remote host",
            ));
        }
        args.parity.cwd = Some(std::env::current_dir()?);
    }
    if args
        .parity
        .cwd
        .as_ref()
        .is_some_and(|path| !path.is_absolute())
    {
        return Err(CliError::Usage(
            "schedule --cwd must be absolute on the target node",
        ));
    }
    if let Some(ttl) = args.parity.expires_in {
        let delta = chrono::TimeDelta::from_std(ttl.duration())
            .map_err(|_range| CliError::Usage("schedule expiry exceeds supported time"))?;
        args.ends_at = Some(
            chrono::Utc::now()
                .checked_add_signed(delta)
                .ok_or(CliError::Usage("schedule expiry exceeds supported time"))?,
        );
    }
    Ok(())
}

pub(super) fn cadence(args: &ScheduleArgs) -> Result<Option<Cadence>, CliError> {
    let expr = match (&args.every, &args.cron) {
        (Some(every), None) => preset(every)?,
        (None, Some(cron)) => cron.parse().map_err(CliError::ParityCron)?,
        (None, None) => return Ok(None),
        (Some(_), Some(_)) => return Err(CliError::Usage("give only one of --every or --cron")),
    };
    let zone = match (&args.parity.timezone, &args.tz) {
        (Some(zone), None) => zone.clone(),
        (None, Some(zone)) => zone.parse().map_err(CliError::ParityAutomationText)?,
        (None, None) => "UTC".parse().map_err(CliError::ParityAutomationText)?,
        (Some(_), Some(_)) => return Err(CliError::Usage("give only one time zone flag")),
    };
    Ok(Some(Cadence::Cron { expr, zone }))
}

fn preset(text: &str) -> Result<CronExpr, CliError> {
    let duration = text
        .parse::<crate::args::parity::WaitBudget>()
        .map_err(|_invalid| CliError::Usage("invalid schedule cadence duration"))?
        .duration();
    let seconds = duration.as_secs();
    let minutes = seconds
        .checked_div(60)
        .filter(|_| seconds.checked_rem(60) == Some(0));
    let hours = seconds
        .checked_div(3600)
        .filter(|_| seconds.checked_rem(3600) == Some(0));
    let expr = match (minutes, hours) {
        (Some(minutes), _) if minutes < 60 && 60_u64.checked_rem(minutes) == Some(0) => {
            format!("*/{minutes} * * * *")
        }
        (_, Some(1)) => "0 * * * *".to_owned(),
        (_, Some(hours)) if hours < 24 && 24_u64.checked_rem(hours) == Some(0) => {
            format!("0 */{hours} * * *")
        }
        (_, Some(24)) => "0 0 * * *".to_owned(),
        _ => {
            return Err(CliError::Usage(
                "--every cannot be represented faithfully by five-field cron; use --cron",
            ));
        }
    };
    expr.parse().map_err(CliError::ParityCron)
}

pub(super) fn place(args: &ScheduleArgs, target: &mut ScheduleTarget) -> Result<(), CliError> {
    let Some(cwd) = &args.parity.cwd else {
        return Ok(());
    };
    match target {
        ScheduleTarget::NewSession { spec, .. } => {
            spec.cwd = Some(cwd.clone());
            spec.place = None;
            Ok(())
        }
        ScheduleTarget::Session { .. } | ScheduleTarget::Unknown => {
            Err(CliError::Usage("--cwd requires a new-agent schedule"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calendar_boundaries_never_change_an_accepted_interval() {
        for (value, expected) in [
            ("1m", "*/1 * * * *"),
            ("15m", "*/15 * * * *"),
            ("1h", "0 * * * *"),
            ("6h", "0 */6 * * *"),
            ("1d", "0 0 * * *"),
        ] {
            assert_eq!(preset(value).unwrap().as_str(), expected);
        }
        for value in ["30s", "7m", "5h", "2d", "15minutes", "junk15m", "1h-nope"] {
            assert!(preset(value).is_err(), "{value}");
        }
    }
}
