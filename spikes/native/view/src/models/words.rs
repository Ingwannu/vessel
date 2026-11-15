//! Typed model-state meanings mapped to prepared Fluent labels.
//! Plain cached getters borrow the catalog; formatting never runs per row.
use super::{Issue, Problem, Refresh, page::StartProblem};
use crate::localization::Models;

impl<'a> Models<'a> {
    /// The refresh control reports pending work without hiding retained rows.
    #[must_use]
    pub fn refresh_label(self, state: Refresh) -> &'a str {
        match state {
            Refresh::Pending => self.refreshing(),
            Refresh::Ready | Refresh::Unavailable => self.refresh(),
        }
    }
    /// Initial failure retry uses the reference's pending label.
    #[must_use]
    pub fn retry_label(self, state: Refresh) -> &'a str {
        match state {
            Refresh::Pending => self.retrying(),
            Refresh::Ready | Refresh::Unavailable => self.retry(),
        }
    }
    /// One complete sentence, never a transport diagnostic inside a translation.
    #[must_use]
    pub fn issue(self, issue: Issue) -> &'a str {
        match issue {
            Issue::Read(problem) => self.problem(problem),
            Issue::Start(problem) => self.start_problem(problem),
        }
    }
    /// Every refused start has its own action in the catalog.
    #[must_use]
    pub fn start_problem(self, problem: StartProblem) -> &'a str {
        match problem {
            StartProblem::NoMachine => self.start_no_machine(),
            StartProblem::Busy => self.start_busy(),
            StartProblem::Exhausted => self.start_exhausted(),
        }
    }
    /// Every typed failure has a complete translated sentence.
    #[must_use]
    pub fn problem(self, problem: Problem) -> &'a str {
        match problem {
            Problem::Offline => self.problem_offline(),
            Problem::Busy => self.problem_busy(),
            Problem::Timeout => self.problem_timeout(),
            Problem::NoTransport => self.problem_no_transport(),
            Problem::Denied => self.problem_denied(),
            Problem::Unsupported => self.problem_unsupported(),
            Problem::Failed => self.problem_failed(),
        }
    }
    /// Use the public product name instead of the internal wire spelling.
    #[must_use]
    pub fn api_name(self, api: vessel_proto::ModelApi) -> &'a str {
        match api {
            vessel_proto::ModelApi::AnthropicMessages => self.api_anthropic(),
            vessel_proto::ModelApi::OpenAiResponses => self.api_responses(),
            vessel_proto::ModelApi::OpenAiChatCompletions => self.api_chat(),
        }
    }
}
#[cfg(test)]
mod tests;
