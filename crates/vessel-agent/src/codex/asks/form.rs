//! Blocking Codex forms retain choices, custom companions and presentation metadata.

use crate::codex::protocol::UserInputRequest;
use crate::mapper::cut;
use vessel_proto::conversation::{ChoiceOption, FieldKind, FieldName, QuestionField};
use vessel_proto::timeline::{
    EmptyAnswerPolicy, QuestionDeliveryState, QuestionFieldPresentation, QuestionLifetime,
};
use vessel_proto::{Question, QuestionId, ToolCallId};

pub(in crate::codex) fn user_input_question(
    asked: &UserInputRequest,
) -> Result<Question, &'static str> {
    if asked.questions.is_empty() || asked.questions.len() > Question::MAX_FIELDS {
        return Err("question count exceeds the complete form bound");
    }
    if asked.questions.iter().any(|question| question.is_secret) {
        return Err("it asks for a secret, which the timeline must not keep");
    }
    let mut fields = Vec::new();
    for question in &asked.questions {
        let name: FieldName = question
            .id
            .parse()
            .map_err(|_invalid| "question id cannot name a field")?;
        let options = question.options.as_deref().unwrap_or_default();
        if options.len() > Question::MAX_OPTIONS {
            return Err("option count exceeds the complete form bound");
        }
        let companion: FieldName = format!("{}.__custom", question.id)
            .parse()
            .map_err(|_invalid| "custom companion id cannot name a field")?;
        let custom = !options.is_empty() && (question.is_other || question.allow_other);
        let kind = if options.is_empty() {
            text()
        } else {
            FieldKind::Choice {
                options: options
                    .iter()
                    .map(|option| ChoiceOption {
                        value: option.label.clone(),
                        title: cut(option.label.clone(), Question::MAX_LABEL),
                        description: option
                            .description
                            .clone()
                            .map(|text| cut(text, Question::MAX_LABEL)),
                        preview: option
                            .preview
                            .clone()
                            .map(|text| cut(text, Question::MAX_LABEL)),
                    })
                    .collect(),
                multiple: question.multi_select,
                min_items: None,
                max_items: None,
                default: vec![],
            }
        };
        fields.push(QuestionField {
            name,
            title: Some(cut(question.header.clone(), Question::MAX_LABEL)),
            description: Some(cut(question.question.clone(), Question::MAX_LABEL)),
            required: true,
            kind,
            presentation: Some(QuestionFieldPresentation {
                custom_answer: custom.then(|| companion.clone()),
                placeholder: question
                    .placeholder
                    .clone()
                    .map(|text| cut(text, Question::MAX_LABEL)),
                empty: if question.allow_empty {
                    EmptyAnswerPolicy::AllowEmpty
                } else {
                    EmptyAnswerPolicy::RequireText
                },
                dismiss_label: question
                    .dismiss_label
                    .clone()
                    .map(|text| cut(text, Question::MAX_LABEL)),
            }),
        });
        if custom {
            fields.push(QuestionField {
                name: companion,
                title: Some("Custom answer".into()),
                description: None,
                required: false,
                kind: text(),
                presentation: None,
            });
        }
    }
    if fields.len() > Question::MAX_FIELDS {
        return Err("custom companions exceed the complete form bound");
    }
    let question = Question {
        tool_scope: None,
        id: QuestionId::random(),
        message: cut(
            asked
                .questions
                .iter()
                .map(|question| question.question.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            Question::MAX_MESSAGE,
        ),
        tool_call: Some(ToolCallId::new(asked.item_id.as_str())),
        fields,
        lifetime: QuestionLifetime::Turn,
        delivery_state: QuestionDeliveryState::Pending,
    };
    question
        .validate_presentation()
        .map_err(|_invalid| "custom companion relationship is ambiguous")?;
    Ok(question)
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
