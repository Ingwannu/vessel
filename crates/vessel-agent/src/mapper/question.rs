//! A form elicitation (`elicitation/create`) as a [`Question`], and how a
//! question was settled as the agent's answer (protocol 10). Pure, and
//! bounded on the way in.
//!
//! [Decision Log]
//! - Purpose: the form is JSON Schema on ACP's side; a client gets typed
//!   fields, and the node checks an answer against them before the agent
//!   sees it (`Question::check`).
//! - Bounds (the contract's): a message cut at [`Question::MAX_MESSAGE`]
//!   bytes, titles and descriptions at [`Question::MAX_LABEL`], at most
//!   [`Question::MAX_OPTIONS`] options a field (the rest left out, with a
//!   warning). Option values and defaults are never cut: a cut value would
//!   be an answer the agent never offered.
//! - A form Vessel cannot show whole is declined at once ([`Unfit`]): more
//!   than [`Question::MAX_FIELDS`] fields (cutting fields would ask a
//!   different question), or a required field the form does not define, or
//!   names in a way Vessel cannot (no answer could ever satisfy it). An
//!   optional field whose name Vessel cannot carry is left out, with a
//!   trace: the rest of the form still asks what the agent meant.
//! - A property kind newer than this build's ACP is [`FieldKind::Unknown`]:
//!   shown, never filled (ACP: a client must not render it as a known
//!   control), so the person can only decline.
//! - A default that is not one of a choice's options is dropped, so a
//!   client never preselects an answer the node would refuse.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use agent_client_protocol::v1::schema as acp;
use vessel_proto::conversation::{
    AnswerValue, ChoiceOption, FieldKind, FieldName, FiniteNumber, QuestionField, TextFormat,
};
use vessel_proto::{Question, QuestionId, QuestionOutcome, ToolCallId};

use super::report::cut;

/// Why an `elicitation/create` is not asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// Not a form: a link to open (URL mode) or a mode newer than this
    /// build's ACP (`mode` names it, cut). Vessel advertises neither.
    NotAForm {
        /// The mode, for the log.
        mode: String,
    },
    /// A form Vessel cannot show whole; declined.
    Unfit(Unfit),
}

/// Why a form cannot be shown whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Unfit {
    /// More fields than a question keeps.
    TooManyFields(usize),
    /// A required field the form does not define, or names in a way
    /// Vessel cannot carry (its name, cut).
    Required(String),
}

impl fmt::Display for Unfit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyFields(count) => write!(
                f,
                "it has {count} fields, more than the {} a question keeps",
                Question::MAX_FIELDS
            ),
            Self::Required(name) => write!(
                f,
                "it requires a field `{name}` that it does not define in a form Vessel can show"
            ),
        }
    }
}

/// The longest mode or field name a refusal quotes, in bytes.
const MAX_QUOTED: usize = 64;

/// The question `request` asks, under `id`.
pub(crate) fn question(
    id: QuestionId,
    request: acp::CreateElicitationRequest,
) -> Result<Question, Refusal> {
    let form = match request.mode {
        acp::ElicitationMode::Form(form) => form,
        acp::ElicitationMode::Url(_) => {
            return Err(Refusal::NotAForm {
                mode: "url".to_owned(),
            });
        }
        acp::ElicitationMode::Other(other) => {
            return Err(Refusal::NotAForm {
                mode: cut(other.mode, MAX_QUOTED),
            });
        }
        _ => {
            return Err(Refusal::NotAForm {
                mode: "unknown".to_owned(),
            });
        }
    };
    let tool_call = match form.scope {
        acp::ElicitationScope::Session(scope) => scope
            .tool_call_id
            .map(|call| ToolCallId::new(Arc::clone(&call.0))),
        acp::ElicitationScope::Request(_) | _ => None,
    };
    let schema = form.requested_schema;
    let offered = schema.properties.len();
    if offered > Question::MAX_FIELDS {
        return Err(Refusal::Unfit(Unfit::TooManyFields(offered)));
    }
    let required = schema.required.unwrap_or_default();
    if let Some(undefined) = required
        .iter()
        .find(|name| !schema.properties.contains_key(name.as_str()))
    {
        return Err(Refusal::Unfit(Unfit::Required(cut(
            undefined.clone(),
            MAX_QUOTED,
        ))));
    }
    let mut fields = Vec::with_capacity(offered);
    for (key, property) in schema.properties {
        let needed = required.contains(&key);
        match key.parse::<FieldName>() {
            Ok(name) => fields.push(field(name, property, needed)),
            Err(_) if needed => {
                return Err(Refusal::Unfit(Unfit::Required(cut(key, MAX_QUOTED))));
            }
            Err(error) => tracing::debug!(%error, "a question's optional field left out"),
        }
    }
    Ok(Question {
        tool_scope: None,
        id,
        message: cut(request.message, Question::MAX_MESSAGE),
        tool_call,
        fields,

        lifetime: vessel_proto::timeline::QuestionLifetime::Turn,
        delivery_state: vessel_proto::timeline::QuestionDeliveryState::Pending,
    })
}

fn field(
    name: FieldName,
    property: acp::ElicitationPropertySchema,
    required: bool,
) -> QuestionField {
    use acp::ElicitationPropertySchema as P;
    let (title, description, kind) = match property {
        P::String(text) => (
            text.title,
            text.description,
            string(
                text.enum_values,
                text.one_of,
                Text {
                    min_length: text.min_length,
                    max_length: text.max_length,
                    format: text.format,
                    pattern: text.pattern,
                    default: text.default,
                },
            ),
        ),
        P::Number(number) => (
            number.title,
            number.description,
            FieldKind::Number {
                minimum: number.minimum.and_then(FiniteNumber::new),
                maximum: number.maximum.and_then(FiniteNumber::new),
                default: number.default.and_then(FiniteNumber::new),
            },
        ),
        P::Integer(integer) => (
            integer.title,
            integer.description,
            FieldKind::Integer {
                minimum: integer.minimum,
                maximum: integer.maximum,
                default: integer.default,
            },
        ),
        P::Boolean(boolean) => (
            boolean.title,
            boolean.description,
            FieldKind::Boolean {
                default: boolean.default,
            },
        ),
        P::Array(several) => (
            several.title,
            several.description,
            multiple(
                several.items,
                several.min_items,
                several.max_items,
                several.default.unwrap_or_default(),
            ),
        ),
        P::Other(_) | _ => (None, None, FieldKind::Unknown),
    };
    QuestionField {
        name,
        title: title.map(label),
        description: description.map(label),
        required,
        kind,

        presentation: None,
    }
}

/// A free-text field's constraints, as ACP gives them.
struct Text {
    min_length: Option<u32>,
    max_length: Option<u32>,
    format: Option<acp::StringFormat>,
    pattern: Option<String>,
    default: Option<String>,
}

/// A string property: a choice of one when it lists values (titled ones
/// first), free text otherwise.
fn string(
    values: Option<Vec<String>>,
    titled: Option<Vec<acp::EnumOption>>,
    text: Text,
) -> FieldKind {
    let options = match (titled, values) {
        (Some(titled), _) => Some(options(titled.into_iter().map(choice_option))),
        (None, Some(values)) => Some(options(values.into_iter().map(untitled_option))),
        (None, None) => None,
    };
    match options {
        Some(options) => {
            let default = offered_only(&options, text.default.into_iter().collect());
            FieldKind::Choice {
                options,
                multiple: false,
                min_items: None,
                max_items: None,
                default,
            }
        }
        None => FieldKind::Text {
            min_length: text.min_length,
            max_length: text.max_length,
            format: text.format.and_then(text_format),
            pattern: text.pattern,
            default: text.default,
        },
    }
}

/// A multi-select property; items of a kind newer than this build's ACP
/// make it [`FieldKind::Unknown`].
fn multiple(
    items: acp::MultiSelectItems,
    min_items: Option<u64>,
    max_items: Option<u64>,
    default: Vec<String>,
) -> FieldKind {
    let options = match items {
        acp::MultiSelectItems::String(items) => {
            options(items.values.into_iter().map(untitled_option))
        }
        acp::MultiSelectItems::Titled(items) => {
            options(items.options.into_iter().map(choice_option))
        }
        acp::MultiSelectItems::Other(_) | _ => return FieldKind::Unknown,
    };
    let default = offered_only(&options, default);
    FieldKind::Choice {
        options,
        multiple: true,
        // Saturating: a bound past `u32` is no bound a person can reach.
        min_items: min_items.map(|count| u32::try_from(count).unwrap_or(u32::MAX)),
        max_items: max_items.map(|count| u32::try_from(count).unwrap_or(u32::MAX)),
        default,
    }
}

/// At most [`Question::MAX_OPTIONS`] options, in the agent's order.
fn options(all: impl ExactSizeIterator<Item = ChoiceOption>) -> Vec<ChoiceOption> {
    let offered = all.len();
    if offered > Question::MAX_OPTIONS {
        tracing::warn!(
            offered,
            kept = Question::MAX_OPTIONS,
            "a question offers more options than a field keeps"
        );
    }
    all.take(Question::MAX_OPTIONS).collect()
}

fn choice_option(option: acp::EnumOption) -> ChoiceOption {
    ChoiceOption {
        value: option.value,
        title: label(option.title),
        description: option.description.map(label),

        preview: None,
    }
}

fn untitled_option(value: String) -> ChoiceOption {
    ChoiceOption {
        title: label(value.clone()),
        value,
        description: None,

        preview: None,
    }
}

/// `chosen`, without the values `options` does not offer.
fn offered_only(options: &[ChoiceOption], mut chosen: Vec<String>) -> Vec<String> {
    chosen.retain(|value| options.iter().any(|option| &option.value == value));
    chosen
}

fn text_format(format: acp::StringFormat) -> Option<TextFormat> {
    match format {
        acp::StringFormat::Email => Some(TextFormat::Email),
        acp::StringFormat::Uri => Some(TextFormat::Uri),
        acp::StringFormat::Date => Some(TextFormat::Date),
        acp::StringFormat::DateTime => Some(TextFormat::DateTime),
        // A newer format: the text is still text, unchecked by shape.
        _ => None,
    }
}

fn label(text: String) -> String {
    cut(text, Question::MAX_LABEL)
}

/// The agent's answer to a question settled as `outcome`.
pub(crate) fn elicitation_response(outcome: &QuestionOutcome) -> acp::CreateElicitationResponse {
    let action = match outcome {
        QuestionOutcome::Answered { values } => {
            let content: BTreeMap<String, acp::ElicitationContentValue> = values
                .iter()
                .map(|(name, value)| (name.as_str().to_owned(), content_value(value)))
                .collect();
            acp::ElicitationAction::Accept(acp::ElicitationAcceptAction::new().content(content))
        }
        QuestionOutcome::Declined => acp::ElicitationAction::Decline,
        QuestionOutcome::Cancelled => acp::ElicitationAction::Cancel,
    };
    acp::CreateElicitationResponse::new(action)
}

fn content_value(value: &AnswerValue) -> acp::ElicitationContentValue {
    match value {
        AnswerValue::Text(text) | AnswerValue::Choice(text) => {
            acp::ElicitationContentValue::String(text.clone())
        }
        AnswerValue::Number(number) => acp::ElicitationContentValue::Number(number.get()),
        AnswerValue::Integer(number) => acp::ElicitationContentValue::Integer(*number),
        AnswerValue::Boolean(yes) => acp::ElicitationContentValue::Boolean(*yes),
        AnswerValue::Choices(values) => acp::ElicitationContentValue::StringArray(values.clone()),
    }
}

#[cfg(test)]
mod tests;
