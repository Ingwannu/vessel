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

impl<'a> AgentCatalog<'a> {
    /// Form validation failures have one complete instruction per cause.
    #[must_use]
    pub fn form_problem(self, error: &crate::agent_status::addition::Refused) -> &'a str {
        use crate::agent_status::{addition::Refused, pages::StartProblem};
        match error {
            Refused::Name(_) => self.invalid_name(),
            Refused::Taken => self.name_taken(),
            Refused::Program(_) => self.invalid_program(),
            Refused::MachineChanged | Refused::Changed => self.form_changed(),
            Refused::Unavailable => self.form_unavailable(),
            Refused::Request(StartProblem::Busy | StartProblem::Capacity) => self.busy(),
            Refused::Request(StartProblem::Exhausted) => self.form_exhausted(),
        }
    }
    /// Administrative errors do not reuse removal or read instructions.
    #[must_use]
    pub fn addition_failure(self, error: crate::agent_status::addition::Failure) -> &'a str {
        use crate::agent_status::addition::Failure;
        match error {
            Failure::Changed => self.add_changed(),
            Failure::Unsupported => self.add_unsupported(),
            Failure::BuildUnavailable => self.add_unassigned(),
            Failure::Exhausted => self.form_exhausted(),
            Failure::Permission => self.add_permission(),
            Failure::Conflict => self.add_conflict(),
            Failure::Rejected => self.add_rejected(),
            Failure::Unknown => self.add_unknown(),
            Failure::Offline => self.add_offline(),
            Failure::Busy => self.busy(),
            Failure::NoTransport => self.no_transport(),
        }
    }
}

impl<'a> AgentCatalog<'a> {
    /// Correctly name the node's supplied resource, not an inferred install page.
    #[must_use]
    pub fn resource_label(self, resource: &super::Resource<'_>) -> &'a str {
        match resource {
            super::Resource::Website(_) => self.website(),
            super::Resource::Repository(_) => self.repository(),
            super::Resource::Archive(_) => self.download_link(),
        }
    }
}

impl<'a> AgentCatalog<'a> {
    /// A stale confirmation is a local refusal; no removal frame was sent.
    #[must_use]
    pub fn removal_refused(self, error: crate::agent_status::removal::Refused) -> &'a str {
        use crate::agent_status::{pages::StartProblem, removal::Refused};
        match error {
            Refused::RefreshRequired => self.remove_refresh(),
            Refused::Unavailable => self.remove_unavailable(),
            Refused::MachineChanged | Refused::NodeChanged | Refused::DefinitionChanged => {
                self.remove_changed()
            }
            Refused::Request(StartProblem::Busy | StartProblem::Capacity) => self.busy(),
            Refused::Request(StartProblem::Exhausted) => self.form_exhausted(),
        }
    }
}

impl<'a> AgentCatalog<'a> {
    /// The same typed notice chooses this whole sentence and its alert tone.
    #[must_use]
    pub fn removal_notice(self, notice: crate::agent_status::removal::Notice) -> &'a str {
        match notice {
            crate::agent_status::removal::Notice::Words => self.remove_words_failed(),
            crate::agent_status::removal::Notice::Refused(problem) => self.removal_refused(problem),
        }
    }
}
