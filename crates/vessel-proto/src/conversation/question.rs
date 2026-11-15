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

use serde::{Deserialize, Serialize};

use super::agent::FiniteNumber;
use super::text::FieldName;
use crate::id::{QuestionId, ToolCallId};
use crate::timeline::{QuestionDeliveryState, QuestionFieldPresentation, QuestionLifetime};

mod check;
mod presentation;

pub use check::InvalidAnswer;
pub use presentation::{InvalidQuestionPresentation, QuestionPresentationError};

/// A question waiting for an answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Question {
    /// Async question lifetime is independent of its completed provider tool.
    #[serde(default, skip_serializing_if = "QuestionLifetime::is_turn")]
    pub lifetime: QuestionLifetime,
    /// Only Pending is actionable; an uncertain accepted answer cannot be resent.
    #[serde(default, skip_serializing_if = "QuestionDeliveryState::is_pending")]
    pub delivery_state: QuestionDeliveryState,
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
    /// Generic custom/empty/dismiss semantics, without provider-name parsing in clients.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<QuestionFieldPresentation>,
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
    /// Optional option preview is displayed without submitting its contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
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
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod presentation_tests;
