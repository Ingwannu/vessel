//! Plan decisions use one title in their pending dock and stream card.

use super::ConversationWords;
use crate::timeline::PlanOutcome;

impl ConversationWords {
    /// A settled plan keeps its decision visible after its buttons disappear.
    #[must_use]
    pub const fn plan_title(&self, outcome: PlanOutcome) -> &'static str {
        match outcome {
            PlanOutcome::Proposed | PlanOutcome::Pending => self.permission_plan(),
            PlanOutcome::Approved => self.either("Approved plan", "승인된 계획"),
            PlanOutcome::Rejected => self.either("Rejected plan", "거절된 계획"),
            PlanOutcome::Canceled => self.either("Canceled plan", "취소된 계획"),
        }
    }
}
