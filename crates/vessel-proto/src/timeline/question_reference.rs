//! Bounded pointer attachment reads the authoritative extended question shape.
use super::{QuestionDeliveryState, QuestionLifetime};
use serde::{Deserialize, Serialize};

/// Oversized inbox requests retain lifetime/actionability even without their full form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QuestionReference {
    /// A turn ending must not erase a retained session request.
    pub lifetime: QuestionLifetime,
    /// Only Pending permits another answer attempt.
    pub delivery_state: QuestionDeliveryState,
}

impl From<&crate::Question> for QuestionReference {
    fn from(question: &crate::Question) -> Self {
        Self {
            lifetime: question.lifetime,
            delivery_state: question.delivery_state,
        }
    }
}
