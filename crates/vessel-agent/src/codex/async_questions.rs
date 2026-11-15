//! Codex completed async agent messages describe questions without a live responder.
//! Stable provider item identity is retained by the node, not in an unbounded driver map.

use super::protocol::ThreadItem;
use serde::Deserialize;
use vessel_proto::conversation::{ChoiceOption, FieldKind, FieldName, QuestionField};
use vessel_proto::timeline::{
    EmptyAnswerPolicy, QuestionDeliveryState, QuestionFieldPresentation, QuestionLifetime,
};
use vessel_proto::{Question, QuestionId, ToolCallId};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Delivery {
    Async,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct Asked {
    title: String,
    #[serde(default)]
    options: Option<Vec<String>>,
}

pub(super) fn question(item: &ThreadItem) -> Result<Option<Question>, &'static str> {
    let ThreadItem::AgentMessage {
        id,
        delivery: Some(Delivery::Async),
        questions,
        ..
    } = item
    else {
        return Ok(None);
    };
    // Delivery=async also describes ordinary assistant messages. Only a real
    // nonempty question collection enters this lifecycle (Paseo ItemSchema).
    let Some(questions) = questions.as_ref().filter(|questions| !questions.is_empty()) else {
        return Ok(None);
    };
    if id.is_empty()
        || questions
            .len()
            .checked_mul(2)
            .is_none_or(|fields| fields > Question::MAX_FIELDS)
    {
        return Err("async question identity or field count is invalid");
    }
    let mut fields = Vec::new();
    for (index, asked) in questions.iter().enumerate() {
        let title = vessel_proto::trim_ecmascript(&asked.title);
        if title.is_empty() {
            return Err("async question title is empty");
        }
        let name: FieldName = format!("question-{index}")
            .parse()
            .map_err(|_invalid| "async field name is invalid")?;
        let choices = asked.options.as_deref().unwrap_or_default();
        if choices.len() > Question::MAX_OPTIONS || choices.iter().any(String::is_empty) {
            return Err("async options are invalid or exceed the question bound");
        }
        let custom: FieldName = format!("question-{index}-custom")
            .parse()
            .map_err(|_invalid| "async custom field name is invalid")?;
        let kind = if choices.is_empty() {
            text()
        } else {
            FieldKind::Choice {
                options: choices
                    .iter()
                    .map(|label| ChoiceOption {
                        value: label.clone(),
                        title: crate::mapper::cut(label.clone(), Question::MAX_LABEL),
                        description: None,
                        preview: None,
                    })
                    .collect(),
                multiple: false,
                min_items: None,
                max_items: None,
                default: vec![],
            }
        };
        fields.push(QuestionField {
            name,
            title: Some(format!("Question {}", index.saturating_add(1))),
            description: Some(crate::mapper::cut(title.to_owned(), Question::MAX_LABEL)),
            required: true,
            kind,
            presentation: Some(QuestionFieldPresentation {
                custom_answer: (!choices.is_empty()).then(|| custom.clone()),
                placeholder: None,
                empty: EmptyAnswerPolicy::RequireText,
                dismiss_label: None,
            }),
        });
        if !choices.is_empty() {
            fields.push(QuestionField {
                name: custom,
                title: Some("Custom answer".into()),
                description: None,
                required: false,
                kind: text(),
                presentation: None,
            });
        }
    }
    let question = Question {
        tool_scope: None,
        id: QuestionId::random(),
        message: crate::mapper::cut(
            questions
                .iter()
                .map(|question| vessel_proto::trim_ecmascript(&question.title))
                .collect::<Vec<_>>()
                .join("\n\n"),
            Question::MAX_MESSAGE,
        ),
        tool_call: Some(ToolCallId::new(id.as_str())),
        fields,
        lifetime: QuestionLifetime::Session,
        delivery_state: QuestionDeliveryState::Pending,
    };
    question
        .validate_presentation()
        .map_err(|_invalid| "async companion relationship is invalid")?;
    Ok(Some(question))
}

fn text() -> FieldKind {
    FieldKind::Text {
        min_length: None,
        max_length: None,
        format: None,
        pattern: None,
        default: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;
    use vessel_proto::QuestionAnswer;
    use vessel_proto::conversation::AnswerValue;

    #[test]
    fn completed_async_items_keep_choices_and_custom_pairing_without_a_turn_responder() {
        let item: ThreadItem = serde_json::from_value(json!({"type":"agentMessage","id":"async","delivery":"async","questions":[{"title":"Where?","options":["Left","Right"]}]})).unwrap();
        let question = question(&item).unwrap().unwrap();
        assert_eq!(question.lifetime, QuestionLifetime::Session);
        assert_eq!(question.tool_call.as_ref().unwrap().as_str(), "async");
        assert!(matches!(question.fields[0].kind, FieldKind::Choice { .. }));
        let companion = question.fields[0]
            .presentation
            .as_ref()
            .unwrap()
            .custom_answer
            .clone()
            .unwrap();
        assert!(
            question
                .check(&QuestionAnswer::Accept {
                    values: BTreeMap::from([(companion, AnswerValue::Text("Elsewhere".into()))])
                })
                .is_ok()
        );
    }

    #[test]
    fn malformed_async_items_never_become_actionable_questions() {
        let item: ThreadItem = serde_json::from_value(json!({"type":"agentMessage","id":"async","delivery":"async","questions":[{"title":"  "}]})).unwrap();
        assert!(question(&item).is_err());
        let ordinary: ThreadItem =
            serde_json::from_value(json!({"type":"agentMessage","id":"normal","text":"hello"}))
                .unwrap();
        assert!(question(&ordinary).unwrap().is_none());
    }

    #[test]
    fn asynchronous_delivery_without_a_form_remains_an_ordinary_message() {
        for questions in [serde_json::Value::Null, json!([])] {
            let item: ThreadItem = serde_json::from_value(json!({"type":"agentMessage","id":"ordinary-async","delivery":"async","text":"Keep this body","questions":questions})).unwrap();
            assert!(question(&item).unwrap().is_none());
        }
    }

    #[test]
    fn native_question_title_uses_ecmascript_trim_without_losing_nel_content() {
        let item: ThreadItem = serde_json::from_value(json!({"type":"agentMessage","id":"async","delivery":"async","questions":[{"title":"\u{feff}Where?\u{feff}"},{"title":"\u{85}"}]})).unwrap();
        let form = question(&item).unwrap().unwrap();
        assert_eq!(form.fields[0].description.as_deref(), Some("Where?"));
        assert_eq!(form.fields[1].description.as_deref(), Some("\u{85}"));
        let blank: ThreadItem = serde_json::from_value(json!({"type":"agentMessage","id":"async","delivery":"async","questions":[{"title":"\u{feff}"}]})).unwrap();
        assert!(question(&blank).is_err());
    }
}
