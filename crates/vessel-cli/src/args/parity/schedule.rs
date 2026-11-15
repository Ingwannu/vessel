//! New-agent schedule flags are decoded separately from legacy saved-session targets.
use super::{WaitBudget, provider_argument::ProviderArgument};
use clap::Args;
use std::path::PathBuf;
use vessel_proto::{OptionValueId, automation::TimeZoneName};

#[derive(Debug, Args)]
pub(crate) struct ScheduleParityArgs {
    #[arg(long, conflicts_with_all = ["harness", "session"])]
    pub(crate) provider: Option<ProviderArgument>,
    #[arg(long, requires = "cron", conflicts_with = "tz")]
    pub(crate) timezone: Option<TimeZoneName>,
    #[arg(long, conflicts_with_all = ["project", "worktree", "session"])]
    pub(crate) cwd: Option<PathBuf>,
    #[arg(long, value_parser = super::identifier::option)]
    pub(crate) mode: Option<OptionValueId>,
    #[arg(long, value_parser = super::identifier::option)]
    pub(crate) thinking: Option<OptionValueId>,
    #[arg(long, hide = true)]
    pub(crate) target: Option<String>,
    #[arg(long)]
    pub(crate) run_now: bool,
    #[arg(long, conflicts_with = "ends_at")]
    pub(crate) expires_in: Option<WaitBudget>,
    #[command(flatten)]
    pub(crate) reset: ResetLimits,
}
#[derive(Debug, Args)]
pub(crate) struct ResetLimits {
    #[arg(long, conflicts_with = "max_runs")]
    pub(crate) no_max_runs: bool,
    #[arg(long, conflicts_with_all = ["expires_in", "ends_at"])]
    pub(crate) no_expires_in: bool,
}
