//! Whether an answer fits its question (protocol 10): only the form's
//! fields, each required one given, each value of its field's kind and
//! within its bounds and options. The node checks every answer here before
//! the agent sees it.

use std::fmt;

use super::{
    AnswerValue, ChoiceOption, FieldKind, FieldName, FiniteNumber, Question, QuestionAnswer,
    QuestionField,
};

/// Why an answer does not fit its question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidAnswer {
    /// The form declares an invalid custom-answer relationship.
    Presentation(super::InvalidQuestionPresentation),
    /// A field the question does not have.
    UnknownField(FieldName),
    /// A required field left out.
    Missing(FieldName),
    /// A value of another kind than the field's, or one outside its
    /// bounds or options.
    Wrong {
        /// Which field.
        field: FieldName,
        /// Why, for people.
        why: &'static str,
    },
    /// The question has a field this build cannot fill.
    Unanswerable,
}

impl fmt::Display for InvalidAnswer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Presentation(error) => error.fmt(f),
            Self::UnknownField(field) => write!(f, "the question has no field `{field}`"),
            Self::Missing(field) => write!(f, "field `{field}` needs an answer"),
            Self::Wrong { field, why } => write!(f, "field `{field}`: {why}"),
            Self::Unanswerable => {
                f.write_str("the question has a field of a kind this node cannot fill; decline it")
            }
        }
    }
}

impl std::error::Error for InvalidAnswer {}

impl Question {
    /// Whether `answer` fits: only its fields, each required one given,
    /// each value of its field's kind and within its bounds and options.
    /// A decline always fits.
    ///
    /// # Errors
    /// The first misfit found, in the form's order.
    pub fn check(&self, answer: &QuestionAnswer) -> Result<(), InvalidAnswer> {
        let QuestionAnswer::Accept { values } = answer else {
            return Ok(());
        };
        self.validate_presentation()
            .map_err(InvalidAnswer::Presentation)?;
        if let Some(unknown) = values
            .keys()
            .find(|name| !self.fields.iter().any(|field| &field.name == *name))
        {
            return Err(InvalidAnswer::UnknownField(unknown.clone()));
        }
        for field in &self.fields {
            match values.get(&field.name) {
                Some(value) => field.check(value)?,
                None if field.required && !super::presentation::custom_satisfies(field, values) => {
                    return Err(InvalidAnswer::Missing(field.name.clone()));
                }
                None => {}
            }
        }
        Ok(())
    }
}

impl QuestionField {
    fn check(&self, value: &AnswerValue) -> Result<(), InvalidAnswer> {
        let fits = match (&self.kind, value) {
            (FieldKind::Unknown, _) => return Err(InvalidAnswer::Unanswerable),
            (
                FieldKind::Text {
                    min_length,
                    max_length,
                    ..
                },
                AnswerValue::Text(text),
            ) => {
                if !super::presentation::text_is_allowed(self, text) {
                    Err("the text must contain non-whitespace content")
                } else {
                    text_fits(text, *min_length, *max_length)
                }
            }
            (
                FieldKind::Number {
                    minimum, maximum, ..
                },
                AnswerValue::Number(number),
            ) => within(
                number.get(),
                minimum.map(FiniteNumber::get),
                maximum.map(FiniteNumber::get),
            ),
            (
                FieldKind::Integer {
                    minimum, maximum, ..
                },
                AnswerValue::Integer(number),
            ) => within(*number, *minimum, *maximum),
            (FieldKind::Boolean { .. }, AnswerValue::Boolean(_)) => Ok(()),
            (
                FieldKind::Choice {
                    options,
                    multiple: false,
                    ..
                },
                AnswerValue::Choice(chosen),
            ) => offered(options, std::slice::from_ref(chosen)),
            (
                FieldKind::Choice {
                    options,
                    multiple: true,
                    min_items,
                    max_items,
                    ..
                },
                AnswerValue::Choices(chosen),
            ) => offered(options, chosen).and_then(|()| {
                let count = u32::try_from(chosen.len()).unwrap_or(u32::MAX);
                if min_items.is_some_and(|min| count < min) {
                    Err("fewer options than the field needs")
                } else if max_items.is_some_and(|max| count > max) {
                    Err("more options than the field allows")
                } else {
                    Ok(())
                }
            }),
            (
                FieldKind::Text { .. }
                | FieldKind::Number { .. }
                | FieldKind::Integer { .. }
                | FieldKind::Boolean { .. }
                | FieldKind::Choice { .. },
                AnswerValue::Text(_)
                | AnswerValue::Number(_)
                | AnswerValue::Integer(_)
                | AnswerValue::Boolean(_)
                | AnswerValue::Choice(_)
                | AnswerValue::Choices(_),
            ) => Err("a value of another kind than the field's"),
        };
        fits.map_err(|why| InvalidAnswer::Wrong {
            field: self.name.clone(),
            why,
        })
    }
}

/// Whether `text` is within the answer limit and the field's lengths
/// (counted in characters, as JSON Schema counts them).
fn text_fits(text: &str, min: Option<u32>, max: Option<u32>) -> Result<(), &'static str> {
    let chars = u32::try_from(text.chars().count()).unwrap_or(u32::MAX);
    if text.len() > Question::MAX_TEXT_ANSWER {
        Err("the text is too long")
    } else if min.is_some_and(|min| chars < min) {
        Err("the text is shorter than the field allows")
    } else if max.is_some_and(|max| chars > max) {
        Err("the text is longer than the field allows")
    } else {
        Ok(())
    }
}

/// Whether `value` is within the bounds the field gives.
fn within<T: PartialOrd + Copy>(
    value: T,
    min: Option<T>,
    max: Option<T>,
) -> Result<(), &'static str> {
    if min.is_some_and(|min| value < min) {
        Err("the number is below the field's minimum")
    } else if max.is_some_and(|max| value > max) {
        Err("the number is above the field's maximum")
    } else {
        Ok(())
    }
}

/// Whether every value `chosen` is one of `options`.
fn offered(options: &[ChoiceOption], chosen: &[String]) -> Result<(), &'static str> {
    if chosen
        .iter()
        .all(|value| options.iter().any(|option| &option.value == value))
    {
        Ok(())
    } else {
        Err("not one of the field's options")
    }
}
