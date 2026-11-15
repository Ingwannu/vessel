//! Generic typed answers become one plain follow-up; no client fabricates provider messages.

use vessel_proto::conversation::{AnswerValue, FieldKind};
use vessel_proto::{Question, QuestionAnswer};

pub(super) fn answer_prompt(question: &Question, answer: &QuestionAnswer) -> Option<String> {
    let QuestionAnswer::Accept { values } = answer else {
        return None;
    };
    let mut prompt = String::from("Answers to your questions:\n\n");
    for field in &question.fields {
        if question.fields.iter().any(|parent| {
            parent
                .presentation
                .as_ref()
                .and_then(|shown| shown.custom_answer.as_ref())
                == Some(&field.name)
        }) {
            continue;
        }
        let custom = field
            .presentation
            .as_ref()
            .and_then(|shown| shown.custom_answer.as_ref())
            .and_then(|name| values.get(name))
            .and_then(|value| {
                if let AnswerValue::Text(text) = value {
                    Some(vessel_proto::trim_ecmascript(text))
                } else {
                    None
                }
            })
            .filter(|text| !text.is_empty());
        let selected = values
            .get(&field.name)
            .map(|value| match value {
                AnswerValue::Text(text) | AnswerValue::Choice(text) => {
                    vessel_proto::trim_ecmascript(text).to_owned()
                }
                AnswerValue::Choices(choices) => choices.join(", "),
                AnswerValue::Number(number) => number.get().to_string(),
                AnswerValue::Integer(number) => number.to_string(),
                AnswerValue::Boolean(value) => value.to_string(),
            })
            .unwrap_or_default();
        let response = match (&field.kind, custom) {
            (FieldKind::Choice { multiple: true, .. }, Some(custom)) if !selected.is_empty() => {
                format!("{selected}, {custom}")
            }
            (
                FieldKind::Text { .. }
                | FieldKind::Number { .. }
                | FieldKind::Integer { .. }
                | FieldKind::Boolean { .. }
                | FieldKind::Choice { .. }
                | FieldKind::Unknown,
                Some(custom),
            ) => custom.to_owned(),
            (
                FieldKind::Text { .. }
                | FieldKind::Number { .. }
                | FieldKind::Integer { .. }
                | FieldKind::Boolean { .. }
                | FieldKind::Choice { .. }
                | FieldKind::Unknown,
                None,
            ) => selected,
        };
        prompt.push_str(
            field
                .description
                .as_deref()
                .or(field.title.as_deref())
                .unwrap_or(field.name.as_str()),
        );
        prompt.push('\n');
        prompt.push_str(&response);
        prompt.push_str("\n\n");
    }
    Some(prompt)
}
