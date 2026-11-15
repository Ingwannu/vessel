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
