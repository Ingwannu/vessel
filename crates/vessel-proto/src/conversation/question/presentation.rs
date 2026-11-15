//! Generic choice/custom pairing is validated without provider-specific field names.

use std::collections::BTreeMap;

use super::{AnswerValue, FieldKind, FieldName, Question, QuestionField};
use crate::timeline::EmptyAnswerPolicy;

/// A form's custom companion must be an optional text field in that same form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("question field `{field}`: {reason}")]
pub struct InvalidQuestionPresentation {
    /// The field whose declared relationship is invalid.
    pub field: FieldName,
    /// Typed relationship failure, without untrusted content.
    pub reason: QuestionPresentationError,
}

/// Invalid relationships are refused before a request becomes actionable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum QuestionPresentationError {
    /// Field identities cannot alias each other.
    #[error("duplicate field identity")]
    DuplicateField,
    /// Only a choice can own a custom text companion.
    #[error("custom answers require a choice field")]
    NotChoice,
    /// The target must be a different member of this form.
    #[error("custom companion is missing or refers to itself")]
    MissingCompanion,
    /// A compulsory companion would invalidate an ordinary selected answer.
    #[error("custom companion must be optional text")]
    NotOptionalText,
    /// A text field cannot belong to multiple choices.
    #[error("custom companion already belongs to another choice")]
    SharedCompanion,
}

impl Question {
    /// Check generic presentation relationships before displaying a provider form.
    ///
    /// # Errors
    /// A companion is absent, aliases a field, has the wrong kind, or is required.
    pub fn validate_presentation(&self) -> Result<(), InvalidQuestionPresentation> {
        for (index, field) in self.fields.iter().enumerate() {
            let error = |reason| InvalidQuestionPresentation {
                field: field.name.clone(),
                reason,
            };
            if self
                .fields
                .iter()
                .take(index)
                .any(|earlier| earlier.name == field.name)
            {
                return Err(error(QuestionPresentationError::DuplicateField));
            }
            let Some(companion) = field
                .presentation
                .as_ref()
                .and_then(|shown| shown.custom_answer.as_ref())
            else {
                continue;
            };
            if !matches!(field.kind, FieldKind::Choice { .. }) {
                return Err(error(QuestionPresentationError::NotChoice));
            }
            let target = self
                .fields
                .iter()
                .find(|candidate| &candidate.name == companion && candidate.name != field.name)
                .ok_or_else(|| error(QuestionPresentationError::MissingCompanion))?;
            if target.required || !matches!(target.kind, FieldKind::Text { .. }) {
                return Err(error(QuestionPresentationError::NotOptionalText));
            }
            if self.fields.iter().take(index).any(|earlier| {
                earlier
                    .presentation
                    .as_ref()
                    .and_then(|shown| shown.custom_answer.as_ref())
                    == Some(companion)
            }) {
                return Err(error(QuestionPresentationError::SharedCompanion));
            }
        }
        Ok(())
    }
}

pub(super) fn custom_satisfies(
    field: &QuestionField,
    values: &BTreeMap<FieldName, AnswerValue>,
) -> bool {
    let Some(shown) = &field.presentation else {
        return false;
    };
    let Some(name) = &shown.custom_answer else {
        return false;
    };
    matches!(values.get(name), Some(AnswerValue::Text(text)) if !crate::trim_ecmascript(text).is_empty() || shown.empty == EmptyAnswerPolicy::AllowEmpty)
}

pub(super) fn text_is_allowed(field: &QuestionField, text: &str) -> bool {
    field.presentation.as_ref().is_none_or(|shown| {
        shown.empty == EmptyAnswerPolicy::AllowEmpty || !crate::trim_ecmascript(text).is_empty()
    })
}
