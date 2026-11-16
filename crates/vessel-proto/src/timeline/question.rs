//! Question presentation and durable lifetime are separate from a provider's turn.

use serde::{Deserialize, Serialize};

use crate::conversation::FieldName;

/// Blocking responders end with their turn; nonblocking questions belong to a session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum QuestionLifetime {
    /// Existing live provider request, withdrawn with the turn.
    #[default]
    Turn,
    /// Durable async request, not settled merely because the turn finished.
    Session,
}

impl QuestionLifetime {
    /// Legacy lifetime is omitted when encoding older-compatible questions.
    #[must_use]
    pub const fn is_turn(&self) -> bool {
        matches!(self, Self::Turn)
    }
}

/// An answer admitted before dispatch cannot be offered again after an uncertain acknowledgement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum QuestionDeliveryState {
    /// The authoritative server can accept an answer.
    #[default]
    Pending,
    /// One admitted answer is in flight; another answer must not dispatch.
    Answering,
    /// An external effect may have happened; automatic resend is forbidden.
    OutcomeUnknown,
}

impl QuestionDeliveryState {
    /// Ordinary questions need no additional state field in their legacy encoding.
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

/// Empty text is an explicit provider rule, not inferred from an absent minimum.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum EmptyAnswerPolicy {
    /// A text answer must contain non-whitespace content.
    #[default]
    RequireText,
    /// Empty text is a valid answer when the field has a text input.
    AllowEmpty,
}

/// Optional field presentation carries generic choice/custom semantics to every client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QuestionFieldPresentation {
    /// Custom-text companion belonging to this choice; validate against the complete form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_answer: Option<FieldName>,
    /// Suggested text is a placeholder, never a submitted default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// Provider's explicit empty-answer rule.
    pub empty: EmptyAnswerPolicy,
    /// Provider's dismiss label, without changing decline semantics by itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dismiss_label: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonblocking_lifetime_does_not_imply_an_answer_was_resolved() {
        let lifetime: QuestionLifetime = serde_json::from_str("\"session\"").unwrap();
        let state = QuestionDeliveryState::default();
        assert_eq!(lifetime, QuestionLifetime::Session);
        assert_eq!(state, QuestionDeliveryState::Pending);
        assert!(serde_json::from_str::<QuestionDeliveryState>("\"completed_tool\"").is_err());
    }
}
