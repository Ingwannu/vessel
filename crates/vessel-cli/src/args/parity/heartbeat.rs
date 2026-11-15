//! Heartbeat cadence changes retain the selected caller session's ownership boundary.
use super::WaitBudget;
use clap::Subcommand;
use std::{str::FromStr, time::Duration};
use vessel_proto::automation::{CronExpr, PromptText, ScheduleName, TimeZoneName};

#[derive(Debug, Subcommand)]
pub(crate) enum HeartbeatCommand {
    Create {
        prompt: PromptText,
        #[arg(long)]
        cron: CronExpr,
        #[arg(long, default_value = "UTC")]
        timezone: TimeZoneName,
        #[arg(long)]
        name: Option<ScheduleName>,
        #[arg(long)]
        max_runs: Option<std::num::NonZeroU32>,
        #[arg(long)]
        expires_in: Option<Lifetime>,
    },
    Update {
        id: ScheduleSelector,
        #[arg(long)]
        cron: CronExpr,
        #[arg(long, default_value = "UTC")]
        timezone: TimeZoneName,
    },
    Delete {
        id: ScheduleSelector,
    },
}
#[derive(Debug, Clone)]
pub(crate) struct ScheduleSelector(String);
impl FromStr for ScheduleSelector {
    type Err = crate::error::CliError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        const MAX_SELECTOR_CHARS: usize = 256;
        if text.is_empty()
            || text.chars().any(char::is_control)
            || text.chars().count() > MAX_SELECTOR_CHARS
        {
            Err(crate::error::CliError::Usage(
                "schedule selector must be bounded text without controls",
            ))
        } else {
            Ok(Self(text.to_owned()))
        }
    }
}
impl ScheduleSelector {
    pub(crate) fn text(&self) -> &str {
        &self.0
    }
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct Lifetime(Duration);
impl FromStr for Lifetime {
    type Err = super::duration::InvalidDuration;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse::<WaitBudget>()
            .map(|budget| Self(budget.duration()))
    }
}
impl Lifetime {
    pub(crate) const fn duration(self) -> Duration {
        self.0
    }
}
