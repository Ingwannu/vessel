use super::*;
use crate::shell::language_tests;
use vessel_proto::conversation::{ChoiceOption, FieldKind, FieldName, QuestionField};
use vessel_proto::{AgentEvent, EventRecord, EventSeq, SessionEvent};

fn text() -> FieldKind {
    FieldKind::Text {
        min_length: None,
        max_length: None,
        format: None,
        pattern: None,
        default: None,
    }
}

fn field(name: &str, kind: FieldKind) -> QuestionField {
    QuestionField {
        name: name.parse::<FieldName>().unwrap(),
        title: None,
        description: None,
        required: false,
        kind,
    }
}

fn prepare(shell: &mut Shell, choice: bool, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let mut fields = vec![field("question_0", text())];
    fields.push(field(
        "question_1",
        if choice {
            FieldKind::Choice {
                options: vec![ChoiceOption {
                    value: "a".into(),
                    title: "A".into(),
                    description: None,
                }],
                multiple: false,
                min_items: None,
                max_items: None,
                default: Vec::new(),
            }
        } else {
            text()
        },
    ));
    if choice {
        fields.push(field("question_1_custom", text()));
    }
    let question = Question {
        id: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        message: "A retained second step".into(),
        tool_call: None,
        fields,
    };
    let change = shell.areas.conversation.timeline.apply(&[EventRecord {
        seq: EventSeq::ZERO,
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: AgentEvent::QuestionAsked(question),
        },
    }]);
    assert_eq!(change.added_from, 0);
    assert_eq!(
        shell.areas.conversation.timeline.waiting_questions().len(),
        1
    );
    shell.question_prepare::<SessionPlace>(window, cx);
    shell.question_show::<SessionPlace>(1, window, cx);
}

#[gpui::test]
fn a_language_change_keeps_the_step_answer_caret_selection_and_focus(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    let setups: [language_tests::Setup; 2] = [
        |shell, window, cx| prepare(shell, true, window, cx),
        |shell, window, cx| prepare(shell, false, window, cx),
    ];
    for setup in setups {
        language_tests::preserves(
            cx,
            setup,
            |shell| &shell.areas.conversation.answering.as_ref().unwrap().field,
            |shell| {
                let answering = shell.areas.conversation.answering.as_ref().unwrap();
                assert_eq!(answering.step, 1);
                assert!(!answering.sent);
            },
        );
    }
}
