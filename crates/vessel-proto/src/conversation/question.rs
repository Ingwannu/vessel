//! Questions an agent asks a person (protocol 10): an ACP form
//! elicitation, typed.
//!
//! [Decision Log]
//! - Purpose: Claude's `AskUserQuestion`, Codex's and `OpenCode`'s questions
//!   reach a client as one typed request with a typed answer, like a
//!   permission request, and the first answer wins.
//! - Constraint: ACP has one way for an agent to ask (`elicitation/create`,
//!   form mode: a message and a JSON-Schema-like form), and Claude's
//!   adapter offers the tool only to a client that advertises form
//!   elicitation (otherwise it disables it). A form is untyped JSON on
//!   ACP's side; AGENTS.md wants every field Vessel branches on typed.
//! - Choice: the form's properties become [`QuestionField`]s of four
//!   kinds (text, number, boolean, choice), each with the constraints ACP
//!   gives; a property of a kind ACP adds later is [`FieldKind::Unknown`],
//!   which a client can only decline. The node checks an answer against
//!   the form ([`Question::check`]) before the agent sees it.
//! - URL elicitations (an agent asking a person to open a link, used for
//!   an MCP server's sign-in) are not part of it: Vessel does not
//!   advertise them, so agents do without them, as they do today.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::agent::FiniteNumber;
use super::text::FieldName;
use crate::id::{QuestionId, ToolCallId};

/// A question waiting for an answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Question {
    /// Node-assigned id clients answer with.
    pub id: QuestionId,
    /// What the agent asks (cut at [`Question::MAX_MESSAGE`] bytes).
    pub message: String,
    /// The tool call it belongs to, when the agent says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<ToolCallId>,
    /// The form, in the agent's order.
    pub fields: Vec<QuestionField>,
}

/// One field of a question's form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QuestionField {
    /// Its key, echoed back in the answer.
    pub name: FieldName,
    /// Its label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A longer explanation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether an accepting answer must give it.
    #[serde(default)]
    pub required: bool,
    /// What kind of value it takes.
    #[serde(flatten)]
    pub kind: FieldKind,
}

/// What a field takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldKind {
    /// Free text.
    Text {
        /// Fewest characters.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_length: Option<u32>,
        /// Most characters.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_length: Option<u32>,
        /// A shape the text must have.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        format: Option<TextFormat>,
        /// A regular expression the text must match, as the agent wrote it.
        /// Passed on for a client to check; the node does not (it would
        /// need a regular-expression engine for one field kind), so the
        /// agent may still refuse an answer that misses it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pattern: Option<String>,
        /// What to start with.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<String>,
    },
    /// A number (ACP's `number`).
    Number {
        /// Smallest allowed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        minimum: Option<FiniteNumber>,
        /// Largest allowed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        maximum: Option<FiniteNumber>,
        /// What to start with.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<FiniteNumber>,
    },
    /// A whole number (ACP's `integer`): kept apart from `number` so a
    /// default or a bound cannot be fractional and large values keep their
    /// precision.
    Integer {
        /// Smallest allowed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        minimum: Option<i64>,
        /// Largest allowed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        maximum: Option<i64>,
        /// What to start with.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<i64>,
    },
    /// Yes or no.
    Boolean {
        /// What to start with.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default: Option<bool>,
    },
    /// One or several of a list.
    Choice {
        /// The options, in the agent's order.
        options: Vec<ChoiceOption>,
        /// Several may be chosen.
        #[serde(default)]
        multiple: bool,
        /// Fewest to choose (several).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_items: Option<u32>,
        /// Most to choose (several).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_items: Option<u32>,
        /// What is chosen to start with: one value for a choice of one,
        /// any number for several.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        default: Vec<String>,
    },
    /// A kind this build does not know (a newer ACP's): a client can only
    /// decline the question.
    #[serde(other)]
    Unknown,
}

/// A shape free text must have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TextFormat {
    /// An email address.
    Email,
    /// A URI.
    Uri,
    /// A date, `YYYY-MM-DD`.
    Date,
    /// A date and time, ISO 8601.
    DateTime,
}

/// One option of a choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ChoiceOption {
    /// What is sent back when it is chosen.
    pub value: String,
    /// Its label.
    pub title: String,
    /// A longer explanation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// An answer to a question (`session.send`'s `answer_question`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "answer", rename_all = "snake_case")]
pub enum QuestionAnswer {
    /// The form, filled in.
    Accept {
        /// Each answered field's value.
        values: BTreeMap<FieldName, AnswerValue>,
    },
    /// The person will not answer.
    Decline,
}

/// The value given for one field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum AnswerValue {
    /// For a text field.
    Text(String),
    /// For a number field.
    Number(FiniteNumber),
    /// For a whole-number field.
    Integer(i64),
    /// For a boolean field.
    Boolean(bool),
    /// For a choice of one.
    Choice(String),
    /// For a choice of several.
    Choices(Vec<String>),
}

/// How a question was settled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum QuestionOutcome {
    /// Answered; the values as given.
    Answered {
        /// Each answered field's value.
        values: BTreeMap<FieldName, AnswerValue>,
    },
    /// The person declined.
    Declined,
    /// Withdrawn: the turn or the session ended first.
    Cancelled,
}

/// Why an answer does not fit its question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidAnswer {
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
    /// The longest message kept, in bytes.
    pub const MAX_MESSAGE: usize = 4096;
    /// The longest title or description kept, in bytes.
    pub const MAX_LABEL: usize = 1024;
    /// The most fields a question may have; an agent asking more is
    /// declined.
    pub const MAX_FIELDS: usize = 32;
    /// The most options a choice keeps.
    pub const MAX_OPTIONS: usize = 64;
    /// The longest text an answer may give, in bytes.
    pub const MAX_TEXT_ANSWER: usize = 16 * 1024;

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
        if let Some(unknown) = values
            .keys()
            .find(|name| !self.fields.iter().any(|field| &field.name == *name))
        {
            return Err(InvalidAnswer::UnknownField(unknown.clone()));
        }
        for field in &self.fields {
            match values.get(&field.name) {
                Some(value) => field.check(value)?,
                None if field.required => return Err(InvalidAnswer::Missing(field.name.clone())),
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
            ) => text_fits(text, *min_length, *max_length),
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

#[cfg(test)]
mod tests;
