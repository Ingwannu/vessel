//! Only Plan decision titles migrate here; permission wording stays with A8.

use super::PlanOutcome;
use crate::localization::TimelinePlan;

impl<'a> TimelinePlan<'a> {
    /// Outcome titles are cached plain Fluent values; no formatting in a frame.
    #[must_use]
    pub fn title(self, outcome: PlanOutcome) -> &'a str {
        match outcome {
            PlanOutcome::Proposed | PlanOutcome::Pending => self.plan(),
            PlanOutcome::Approved => self.approved(),
            PlanOutcome::Rejected => self.rejected(),
            PlanOutcome::Canceled => self.canceled(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PlanOutcome;
    use crate::localization::{ENGLISH, LanguagePreference, Translator, resolve_language};

    #[test]
    fn all_plan_outcomes_keep_their_whole_english_and_korean_titles() {
        let en = Translator::get(ENGLISH).unwrap().timeline_plan();
        let ko = Translator::get(resolve_language(
            LanguagePreference::System,
            Some("ko"),
            None,
        ))
        .unwrap()
        .timeline_plan();
        for (outcome, english, korean) in [
            (PlanOutcome::Proposed, "Plan", "계획"),
            (PlanOutcome::Pending, "Plan", "계획"),
            (PlanOutcome::Approved, "Approved plan", "승인된 계획"),
            (PlanOutcome::Rejected, "Rejected plan", "거절된 계획"),
            (PlanOutcome::Canceled, "Canceled plan", "취소된 계획"),
        ] {
            assert_eq!(en.title(outcome), english);
            assert_eq!(ko.title(outcome), korean);
        }
    }
}
