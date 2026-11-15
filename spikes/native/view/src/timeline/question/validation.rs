//! Form validation identity stays typed; display sentences never drive navigation.
use vessel_proto::Question;
use vessel_proto::conversation::{FieldKind, FieldName, InvalidAnswer};

/// Locate a validation refusal through its typed field identity. Human error
/// wording is presentation only; changing it must not change form navigation.
#[must_use]
pub fn refused_field<'q>(question: &'q Question, refusal: &InvalidAnswer) -> Option<&'q FieldName> {
    let named = match refusal {
        InvalidAnswer::UnknownField(name) | InvalidAnswer::Missing(name) => name,
        InvalidAnswer::Wrong { field, .. } => field,
        InvalidAnswer::Unanswerable => return None,
    };
    question
        .fields
        .iter()
        .find(|field| &field.name == named)
        .map(|field| &field.name)
}

/// Whether a question can be answered here at all.
#[must_use]
pub fn answerable(question: &Question) -> bool {
    question
        .fields
        .iter()
        .all(|field| !matches!(field.kind, FieldKind::Unknown))
}
