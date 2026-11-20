//! Usage selectors; rolling periods have names, not raw second counts.
use crate::selector::SessionSelector;
use clap::{Subcommand, ValueEnum};
use vessel_proto::{EventSeq, HarnessRef, ProfileName};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Period {
    Day,
    Week,
}
#[derive(Debug, Subcommand)]
pub(crate) enum UsageCommand {
    /// Totals on the selected machine over the past day or week.
    Machine {
        #[arg(long, value_enum, default_value = "day")]
        period: Period,
    },
    /// Totals of one agent across profiles over the past day or week.
    Agent {
        harness: HarnessRef,
        /// Only this recorded profile name; omitted includes all profiles.
        #[arg(long)]
        profile: Option<ProfileName>,
        #[arg(long, value_enum, default_value = "day")]
        period: Period,
    },
    /// Retained totals and a page of one session's reports.
    Session {
        session: SessionSelector,
        #[arg(long, value_parser = crate::args::parse_seq)]
        after: Option<EventSeq>,
    },
}
