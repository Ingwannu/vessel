//! Registry eligibility uses whole translated sentences, not machine prose.
use super::{Offer, Runner};
use crate::localization::AgentCatalog;

impl<'a> AgentCatalog<'a> {
    /// A missing runner is actionable help, never a false claim that Add
    /// installs the agent. No raw command or registry prose is interpolated.
    #[must_use]
    pub fn offer(self, offer: &Offer<'_>) -> &'a str {
        match offer {
            Offer::Recorded(_) => self.recorded(),
            Offer::Add(Runner::Found) => self.add_help(),
            Offer::Add(Runner::Missing) => self.runner_missing(),
            Offer::BuiltIn(_) => self.built_in(),
            Offer::Download(_) => self.download(),
            Offer::Unavailable => self.unavailable(),
        }
    }
}

impl<'a> AgentCatalog<'a> {
    /// Whole registry-read instructions, independent from removal errors.
    #[must_use]
    pub fn problem(self, problem: super::Problem) -> &'a str {
        match problem {
            super::Problem::NodeChanged => self.changed(),
            super::Problem::Busy => self.busy(),
            super::Problem::Offline => self.offline(),
            super::Problem::Unsupported => self.older(),
            super::Problem::Permission => self.permission(),
            super::Problem::Timeout => self.timeout(),
            super::Problem::Read => self.read_error(),
            super::Problem::NoTransport => self.no_transport(),
        }
    }
}
