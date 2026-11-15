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
    /// Latest saved sign-in limits of one configured agent profile.
    Limits {
        harness: HarnessRef,
        /// Omitted selects the configured default, not all historical profiles.
        #[arg(long)]
        profile: Option<ProfileName>,
        /// Request a vendor check; requires operate permission.
        #[arg(long)]
        refresh: bool,
    },
    /// Saved sign-in limits tied to a session's actual launch.
    SessionLimits {
        session: SessionSelector,
        #[arg(long)]
        refresh: bool,
    },
    /// Persist this machine's credential-use choice; requires admin permission.
    Lookup {
        #[arg(value_enum)]
        policy: LookupChoice,
    },
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

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum LookupChoice {
    Off,
    On,
}
