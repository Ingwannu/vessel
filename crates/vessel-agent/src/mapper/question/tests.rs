use std::collections::BTreeMap;

use vessel_proto::conversation::{AnswerValue, ChoiceOption, FieldKind, TextFormat};
use vessel_proto::{QuestionAnswer, QuestionId, QuestionOutcome};

use super::*;

fn request(json: &str) -> acp::CreateElicitationRequest {
    serde_json::from_str(json).unwrap()
}

fn asked(json: &str) -> Question {
    question(QuestionId::random(), request(json)).unwrap()
}

/// Trimmed from `claude-agent-acp` 0.100's `AskUserQuestion` form: a
/// single choice, a multi-select with titled options, each followed by its
/// free-text "Other" field, nothing required.
const ASK_USER_QUESTION: &str = r#"{
    "mode": "form", "sessionId": "s", "toolCallId": "toolu_1",
    "message": "Please answer the following questions.",
    "requestedSchema": {"type": "object", "properties": {
        "question_0": {"type": "string", "title": "Database", "description": "Which one?",
            "oneOf": [
                {"const": "Postgres", "title": "Postgres", "description": "relational"},
                {"const": "SQLite", "title": "SQLite",
                 "_meta": {"_claude/askUserQuestionOption": {"preview": "x"}}}
            ]},
        "question_0_custom": {"type": "string", "title": "Other",
            "description": "Type your own answer (optional)."},
        "question_1": {"type": "array", "title": "Extras",
            "items": {"anyOf": [{"const": "cache", "title": "Cache"},
                                {"const": "queue", "title": "Queue"}]}},
        "question_1_custom": {"type": "string", "title": "Other"}
    }}
}"#;

#[test]
fn claudes_questions_become_typed_fields() {
    let question = asked(ASK_USER_QUESTION);
    assert_eq!(question.message, "Please answer the following questions.");
    assert_eq!(question.tool_call, Some(ToolCallId::new("toolu_1")));
    let names: Vec<&str> = question.fields.iter().map(|f| f.name.as_str()).collect();
    // ACP's crate keeps a form's properties by key (a known limit).
    assert_eq!(
        names,
        [
            "question_0",
            "question_0_custom",
            "question_1",
            "question_1_custom"
        ]
    );
    assert!(question.fields.iter().all(|field| !field.required));
    let single = &question.fields[0];
    assert_eq!(single.title.as_deref(), Some("Database"));
    let FieldKind::Choice {
        options,
        multiple: false,
        ..
    } = &single.kind
    else {
        panic!("{single:?}");
    };
    assert_eq!(options.len(), 2);
    assert_eq!(options[0].value, "Postgres");
    assert_eq!(options[0].description.as_deref(), Some("relational"));
    assert!(matches!(
        question.fields[1].kind,
        FieldKind::Text { format: None, .. }
    ));
    let FieldKind::Choice {
        options,
        multiple: true,
        ..
    } = &question.fields[2].kind
    else {
        panic!("{:?}", question.fields[2]);
    };
    let values: Vec<&str> = options.iter().map(|o| o.value.as_str()).collect();
    assert_eq!(values, ["cache", "queue"]);

    // What a person answers fits the form the node built.
    let answer = QuestionAnswer::Accept {
        values: BTreeMap::from([
            (
                "question_0".parse().unwrap(),
                AnswerValue::Choice("SQLite".to_owned()),
            ),
            (
                "question_1".parse().unwrap(),
                AnswerValue::Choices(vec!["queue".to_owned()]),
            ),
        ]),
    };
    assert_eq!(question.check(&answer), Ok(()));
}

#[test]
fn every_kind_keeps_its_constraints_and_defaults() {
    let question = asked(
        r#"{"mode": "form", "sessionId": "s", "message": "Settings",
            "requestedSchema": {"properties": {
                "email": {"type": "string", "format": "email", "minLength": 3,
                          "maxLength": 80, "pattern": "^.+@.+$", "default": "a@b.c"},
                "level": {"type": "string", "enum": ["low", "high"], "default": "medium"},
                "ratio": {"type": "number", "minimum": 0, "maximum": 1, "default": 0.5},
                "count": {"type": "integer", "minimum": 1, "default": 3},
                "sure": {"type": "boolean", "default": true},
                "tags": {"type": "array", "minItems": 1, "maxItems": 2,
                         "items": {"type": "string", "enum": ["a", "b", "c"]},
                         "default": ["b", "z"]},
                "picture": {"type": "image", "accept": "png"}
            }, "required": ["level", "sure"]}}"#,
    );
    let kind = |name: &str| {
        &question
            .fields
            .iter()
            .find(|field| field.name.as_str() == name)
            .unwrap()
            .kind
    };
    assert_eq!(
        *kind("email"),
        FieldKind::Text {
            min_length: Some(3),
            max_length: Some(80),
            format: Some(TextFormat::Email),
            pattern: Some("^.+@.+$".to_owned()),
            default: Some("a@b.c".to_owned()),
        }
    );
    let FieldKind::Choice {
        options, default, ..
    } = kind("level")
    else {
        panic!();
    };
    assert_eq!(options[1].title, "high", "an untitled value is its title");
    assert!(
        default.is_empty(),
        "a default that is not an option is dropped"
    );
    assert_eq!(
        *kind("ratio"),
        FieldKind::Number {
            minimum: FiniteNumber::new(0.0),
            maximum: FiniteNumber::new(1.0),
            default: FiniteNumber::new(0.5),
        }
    );
    assert_eq!(
        *kind("count"),
        FieldKind::Integer {
            minimum: Some(1),
            maximum: None,
            default: Some(3),
        }
    );
    assert_eq!(
        *kind("sure"),
        FieldKind::Boolean {
            default: Some(true)
        }
    );
    assert_eq!(
        *kind("tags"),
        FieldKind::Choice {
            options: ["a", "b", "c"]
                .map(|value| ChoiceOption {
                    value: value.to_owned(),
                    title: value.to_owned(),
                    description: None,
                })
                .to_vec(),
            multiple: true,
            min_items: Some(1),
            max_items: Some(2),
            default: vec!["b".to_owned()],
        }
    );
    assert_eq!(
        *kind("picture"),
        FieldKind::Unknown,
        "a kind ACP adds later"
    );
    let required: Vec<&str> = question
        .fields
        .iter()
        .filter(|field| field.required)
        .map(|field| field.name.as_str())
        .collect();
    assert_eq!(required, ["level", "sure"]);
}

#[test]
fn what_vessel_cannot_show_is_refused() {
    let url = request(
        r#"{"mode": "url", "sessionId": "s", "message": "Sign in",
            "elicitationId": "e1", "url": "https://example.com/login"}"#,
    );
    assert_eq!(
        question(QuestionId::random(), url),
        Err(Refusal::NotAForm {
            mode: "url".to_owned()
        })
    );
    let newer = request(r#"{"mode": "voice", "sessionId": "s", "message": "Say it"}"#);
    assert_eq!(
        question(QuestionId::random(), newer),
        Err(Refusal::NotAForm {
            mode: "voice".to_owned()
        })
    );

    let undefined = request(
        r#"{"mode": "form", "sessionId": "s", "message": "?",
            "requestedSchema": {"properties": {"a": {"type": "boolean"}},
                                "required": ["a", "b"]}}"#,
    );
    assert_eq!(
        question(QuestionId::random(), undefined),
        Err(Refusal::Unfit(Unfit::Required("b".to_owned())))
    );
    let unnamed = request(
        r#"{"mode": "form", "sessionId": "s", "message": "?",
            "requestedSchema": {"properties": {"a\u0007": {"type": "boolean"}},
                                "required": ["a\u0007"]}}"#,
    );
    assert!(matches!(
        question(QuestionId::random(), unnamed),
        Err(Refusal::Unfit(Unfit::Required(_)))
    ));

    let properties: Vec<String> = (0..=Question::MAX_FIELDS)
        .map(|n| format!(r#""f{n}": {{"type": "boolean"}}"#))
        .collect();
    let crowded = request(&format!(
        r#"{{"mode": "form", "sessionId": "s", "message": "?",
             "requestedSchema": {{"properties": {{{}}}}}}}"#,
        properties.join(",")
    ));
    assert_eq!(
        question(QuestionId::random(), crowded),
        Err(Refusal::Unfit(Unfit::TooManyFields(
            Question::MAX_FIELDS + 1
        )))
    );
}

#[test]
fn long_text_is_cut_and_options_bounded() {
    let options: Vec<String> = (0..=Question::MAX_OPTIONS)
        .map(|n| format!(r#""v{n}""#))
        .collect();
    let message = "가".repeat(Question::MAX_MESSAGE);
    let question = asked(&format!(
        r#"{{"mode": "form", "sessionId": "s", "message": "{message}",
             "requestedSchema": {{"properties": {{
                "pick": {{"type": "string", "title": "{}", "enum": [{}]}},
                "a\u0007": {{"type": "boolean"}}
             }}}}}}"#,
        "t".repeat(Question::MAX_LABEL + 10),
        options.join(",")
    ));
    assert!(question.message.len() <= Question::MAX_MESSAGE);
    assert!(question.message.chars().all(|c| c == '가'));
    assert_eq!(
        question.fields.len(),
        1,
        "an optional field without a usable name is left out"
    );
    let pick = &question.fields[0];
    assert_eq!(
        pick.title.as_ref().map(String::len),
        Some(Question::MAX_LABEL)
    );
    let FieldKind::Choice { options, .. } = &pick.kind else {
        panic!();
    };
    assert_eq!(options.len(), Question::MAX_OPTIONS);
}

#[test]
fn an_outcome_is_the_action_acp_expects() {
    let answered = QuestionOutcome::Answered {
        values: BTreeMap::from([
            (
                "name".parse().unwrap(),
                AnswerValue::Text("vessel".to_owned()),
            ),
            (
                "way".parse().unwrap(),
                AnswerValue::Choice("left".to_owned()),
            ),
            ("sure".parse().unwrap(), AnswerValue::Boolean(true)),
            ("count".parse().unwrap(), AnswerValue::Integer(2)),
            (
                "tags".parse().unwrap(),
                AnswerValue::Choices(vec!["a".to_owned()]),
            ),
        ]),
    };
    assert_eq!(
        serde_json::to_value(elicitation_response(&answered)).unwrap(),
        serde_json::json!({
            "action": "accept",
            "content": {"count": 2, "name": "vessel", "sure": true, "tags": ["a"], "way": "left"}
        })
    );
    assert_eq!(
        serde_json::to_value(elicitation_response(&QuestionOutcome::Declined)).unwrap(),
        serde_json::json!({"action": "decline"})
    );
    assert_eq!(
        serde_json::to_value(elicitation_response(&QuestionOutcome::Cancelled)).unwrap(),
        serde_json::json!({"action": "cancel"})
    );
}
