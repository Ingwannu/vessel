use super::*;

fn name(text: &str) -> FieldName {
    text.parse().unwrap()
}

fn question() -> Question {
    Question {
        id: QuestionId::random(),
        message: "Which approach?".to_owned(),
        tool_call: None,
        fields: vec![
            QuestionField {
                name: name("question_1"),
                title: Some("Approach".to_owned()),
                description: None,
                required: true,
                kind: FieldKind::Choice {
                    options: vec![
                        ChoiceOption {
                            value: "a".to_owned(),
                            title: "Rewrite".to_owned(),
                            description: Some("Start over".to_owned()),

                            preview: None,
                        },
                        ChoiceOption {
                            value: "b".to_owned(),
                            title: "Patch".to_owned(),
                            description: None,

                            preview: None,
                        },
                    ],
                    multiple: false,
                    min_items: None,
                    max_items: None,
                    default: Vec::new(),
                },

                presentation: None,
            },
            QuestionField {
                name: name("question_1_custom"),
                title: Some("Other".to_owned()),
                description: None,
                required: false,
                kind: FieldKind::Text {
                    min_length: None,
                    max_length: Some(5),
                    format: None,
                    pattern: None,
                    default: None,
                },

                presentation: None,
            },
            QuestionField {
                name: name("count"),
                title: None,
                description: None,
                required: false,
                kind: FieldKind::Integer {
                    minimum: Some(1),
                    maximum: Some(9),
                    default: None,
                },

                presentation: None,
            },
            QuestionField {
                name: name("tags"),
                title: None,
                description: None,
                required: false,
                kind: FieldKind::Choice {
                    options: vec![ChoiceOption {
                        value: "x".to_owned(),
                        title: "X".to_owned(),
                        description: None,

                        preview: None,
                    }],
                    multiple: true,
                    min_items: Some(1),
                    max_items: None,
                    default: Vec::new(),
                },

                presentation: None,
            },
        ],

        lifetime: QuestionLifetime::Turn,
        delivery_state: QuestionDeliveryState::Pending,
    }
}

fn accept(values: &[(&str, AnswerValue)]) -> QuestionAnswer {
    QuestionAnswer::Accept {
        values: values
            .iter()
            .map(|(key, value)| (name(key), value.clone()))
            .collect(),
    }
}

#[test]
fn a_fitting_answer_and_a_decline_pass() {
    let question = question();
    let answer = accept(&[
        ("question_1", AnswerValue::Choice("b".to_owned())),
        ("count", AnswerValue::Integer(3)),
        ("tags", AnswerValue::Choices(vec!["x".to_owned()])),
    ]);
    assert_eq!(question.check(&answer), Ok(()));
    assert_eq!(question.check(&QuestionAnswer::Decline), Ok(()));
}

#[test]
fn each_misfit_is_named() {
    let question = question();
    assert_eq!(
        question.check(&accept(&[])),
        Err(InvalidAnswer::Missing(name("question_1")))
    );
    assert_eq!(
        question.check(&accept(&[
            ("question_1", AnswerValue::Choice("a".to_owned())),
            ("nope", AnswerValue::Boolean(true)),
        ])),
        Err(InvalidAnswer::UnknownField(name("nope")))
    );
    let wrong = |values: &[(&str, AnswerValue)]| {
        let mut all = vec![("question_1", AnswerValue::Choice("a".to_owned()))];
        all.extend_from_slice(values);
        matches!(
            question.check(&accept(&all)),
            Err(InvalidAnswer::Wrong { .. })
        )
    };
    assert!(wrong(&[(
        "question_1",
        AnswerValue::Choice("z".to_owned())
    )]));
    assert!(wrong(&[(
        "question_1_custom",
        AnswerValue::Text("too long".to_owned())
    )]));
    assert!(wrong(&[(
        "count",
        AnswerValue::Number(FiniteNumber::new(2.5).unwrap())
    )]));
    assert!(wrong(&[("count", AnswerValue::Integer(10))]));
    assert!(wrong(&[("count", AnswerValue::Boolean(true))]));
    assert!(wrong(&[("tags", AnswerValue::Choices(Vec::new()))]));
    assert!(wrong(&[("tags", AnswerValue::Choice("x".to_owned()))]));
}

#[test]
fn an_unknown_kind_can_only_be_declined() {
    let field: QuestionField =
        serde_json::from_str(r#"{"name":"f","required":true,"kind":"color"}"#).unwrap();
    assert_eq!(field.kind, FieldKind::Unknown);
    let question = Question {
        fields: vec![field],
        ..question()
    };
    assert_eq!(
        question.check(&accept(&[("f", AnswerValue::Text("red".to_owned()))])),
        Err(InvalidAnswer::Unanswerable)
    );
    assert_eq!(question.check(&QuestionAnswer::Decline), Ok(()));
}

#[test]
fn answers_have_their_wire_shape() {
    let answer = accept(&[("question_1", AnswerValue::Choice("a".to_owned()))]);
    let json = serde_json::to_string(&answer).unwrap();
    assert_eq!(
        json,
        r#"{"answer":"accept","values":{"question_1":{"type":"choice","value":"a"}}}"#
    );
    assert_eq!(
        serde_json::from_str::<QuestionAnswer>(&json).unwrap(),
        answer
    );
    let field = &question().fields[0];
    let json = serde_json::to_value(field).unwrap();
    assert_eq!(json["kind"], "choice");
    assert_eq!(json["options"][0]["value"], "a");
}
