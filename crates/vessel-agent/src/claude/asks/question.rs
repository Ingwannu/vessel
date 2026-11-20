//! Claude Code's `AskUserQuestion` as a question of the session's, and an
//! answer as the tool's input with its answers (the plan's Claude Code
//! slice 2).
//!
//! [Decision Log]
//! - Purpose: the agent's questions reach the person as every other
//!   agent's do (`QuestionAsked`), answered by any client.
//! - The tool's input names each question by its full text and offers
//!   labelled options, one or several (`multiSelect`). Each question is a
//!   field (`question_<n>`, its header the title, its text the
//!   description): a choice of its options, or text when it offers none.
//! - The answer goes back as Paseo sends it: the tool's own input with
//!   `answers`, keyed by each question's full text, a choice of several as
//!   its labels joined by `, `. A decline denies the tool, saying so.
//! - A stated difference from Paseo: Paseo also lets a person type an
//!   answer that is none of the options ("Other", which the tool's schema
//!   leaves to the host). A Vessel question's field is a choice or text,
//!   not both, so a question with options is a choice of them; offering
//!   both would need a new field kind, a protocol change.
//! - Bounds are the session question's own (`Question::MAX_*`); a question
//!   past them, or an input that does not read, is not shown and the tool
//!   is denied, saying why.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Value, json};
use vessel_proto::conversation::{AnswerValue, ChoiceOption, FieldKind, FieldName, QuestionField};
use vessel_proto::{Question, QuestionId, ToolCallId};

use crate::mapper::cut;

/// What the agent is told when the person declines to answer.
pub(super) const DECLINED: &str = "The person using Vessel declined to answer.";

/// The tool's input, as far as the question needs it.
#[derive(Debug, Deserialize)]
struct Input {
    questions: Vec<Asked>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Asked {
    question: String,
    #[serde(default)]
    header: Option<String>,
    #[serde(default)]
    options: Vec<Offered>,
    #[serde(default)]
    multi_select: bool,
}

#[derive(Debug, Deserialize)]
struct Offered {
    label: String,
    #[serde(default)]
    description: Option<String>,
}

/// A question as shown, and how its answer goes back.
#[derive(Debug, Clone)]
pub(super) struct Form {
    pub(super) question: Question,
    /// Each field's name and the question text its answer is keyed by.
    texts: Vec<(FieldName, String)>,
    /// The tool's input, to which the answers are added.
    input: Value,
}

/// `input` (an `AskUserQuestion` call's, `tool_use_id`) as a question;
/// `Err` with why it cannot be shown.
pub(super) fn form(input: &Value, tool_use_id: &str) -> Result<Form, &'static str> {
    let asked = Input::deserialize(input).map_err(|_unread| "its questions do not read")?;
    if asked.questions.is_empty() {
        return Err("it asks nothing");
    }
    if asked.questions.len() > Question::MAX_FIELDS {
        return Err("it asks more than a question keeps");
    }
    let mut fields = Vec::with_capacity(asked.questions.len());
    let mut texts = Vec::with_capacity(asked.questions.len());
    for (index, asked) in asked.questions.iter().enumerate() {
        if asked.options.len() > Question::MAX_OPTIONS {
            return Err("a question offers more options than a question keeps");
        }
        let name: FieldName = format!("question_{}", index.saturating_add(1))
            .parse()
            .map_err(|_unfit| "a question cannot be named")?;
        let kind = if asked.options.is_empty() {
            FieldKind::Text {
                min_length: None,
                max_length: None,
                format: None,
                pattern: None,
                default: None,
            }
        } else {
            FieldKind::Choice {
                options: asked
                    .options
                    .iter()
                    .map(|offered| ChoiceOption {
                        value: offered.label.clone(),
                        title: cut(offered.label.clone(), Question::MAX_LABEL),
                        description: offered
                            .description
                            .clone()
                            .map(|text| cut(text, Question::MAX_LABEL)),
                    })
                    .collect(),
                multiple: asked.multi_select,
                min_items: None,
                max_items: None,
                default: Vec::new(),
            }
        };
        fields.push(QuestionField {
            name: name.clone(),
            title: asked
                .header
                .clone()
                .map(|header| cut(header, Question::MAX_LABEL)),
            description: Some(cut(asked.question.clone(), Question::MAX_LABEL)),
            required: true,
            kind,
        });
        texts.push((name, asked.question.clone()));
    }
    let message = asked
        .questions
        .iter()
        .map(|asked| asked.question.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Form {
        question: Question {
            id: QuestionId::random(),
            message: cut(message, Question::MAX_MESSAGE),
            tool_call: Some(ToolCallId::new(tool_use_id)),
            fields,
        },
        texts,
        input: input.clone(),
    })
}

impl Form {
    /// The tool's input with `values` as its answers.
    pub(super) fn answered(&self, values: &BTreeMap<FieldName, AnswerValue>) -> Value {
        let answers: serde_json::Map<String, Value> = self
            .texts
            .iter()
            .filter_map(|(name, text)| {
                let given = match values.get(name)? {
                    AnswerValue::Text(text) | AnswerValue::Choice(text) => text.clone(),
                    AnswerValue::Choices(labels) => labels.join(", "),
                    AnswerValue::Number(number) => number.get().to_string(),
                    AnswerValue::Integer(number) => number.to_string(),
                    AnswerValue::Boolean(yes) => yes.to_string(),
                };
                Some((text.clone(), json!(given)))
            })
            .collect();
        let mut input = self.input.clone();
        if let Some(fields) = input.as_object_mut() {
            let _old = fields.insert("answers".to_owned(), Value::Object(answers));
        }
        input
    }
}
