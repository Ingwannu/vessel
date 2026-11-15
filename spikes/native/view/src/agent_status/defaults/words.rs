//! Complete defaults-refusal sentences through the generated Fluent scope.
use super::SaveProblem;
use crate::localization::AgentDefaults;

impl<'a> AgentDefaults<'a> {
    /// A failed choice does not replace the previous saved defaults.
    #[must_use]
    pub fn problem(self, problem: SaveProblem) -> &'a str {
        match problem {
            SaveProblem::NotOffered => self.not_offered(),
            SaveProblem::Conflicting => self.conflicting(),
            SaveProblem::Storage(_) => self.storage(),
        }
    }
}
