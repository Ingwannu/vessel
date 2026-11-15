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

pub(super) fn prepare(
    shell: &mut Shell,
    choice: bool,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
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
    fields.push(field("question_2", text()));
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
fn a_single_pick_advances_without_submitting_and_keeps_the_next_draft(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, window, cx| {
            prepare(shell, true, window, cx);
            shell.question_pick::<SessionPlace>("a", window, cx);
            let answering = shell.areas.conversation.answering.as_ref().unwrap();
            assert_eq!(answering.step, 2);
            assert_eq!(answering.sending, None);
            assert_eq!(
                answering.drafts.get(&"question_1".parse().unwrap()),
                Some(&Draft::Picked(vec!["a".to_owned()]))
            );
        },
        |shell| &shell.areas.conversation.answering.as_ref().unwrap().field,
        |shell| {
            let answering = shell.areas.conversation.answering.as_ref().unwrap();
            assert_eq!(answering.step, 2);
            assert_eq!(answering.sending, None);
        },
    );
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
                assert_eq!(answering.sending, None);
            },
        );
    }
}

#[gpui::test]
fn an_old_question_editor_cannot_change_or_submit_its_successor(cx: &mut gpui::TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell
            .areas
            .conversation
            .open("scratch", vessel_proto::SessionId::random());
        prepare(shell, false, window, cx);
        let old = shell.areas.conversation.answering.as_ref().unwrap();
        let source = old.field.id();
        let cursor = old.cursor();
        let previous = old.id;
        let successor = Question {
            id: QuestionId::random(),
            message: "Next request".into(),
            tool_call: None,
            fields: vec![field("answer", text())],
        };
        let _change = shell.areas.conversation.timeline.apply(&[
            EventRecord {
                seq: EventSeq::new(1),
                at: chrono::DateTime::UNIX_EPOCH,
                event: SessionEvent::Agent {
                    event: AgentEvent::QuestionResolved {
                        id: previous,
                        outcome: vessel_proto::QuestionOutcome::Cancelled,
                    },
                },
            },
            EventRecord {
                seq: EventSeq::new(2),
                at: chrono::DateTime::UNIX_EPOCH,
                event: SessionEvent::Agent {
                    event: AgentEvent::QuestionAsked(successor),
                },
            },
        ]);
        shell.question_prepare::<SessionPlace>(window, cx);
        let current_source = shell
            .areas
            .conversation
            .answering
            .as_ref()
            .unwrap()
            .field
            .id();
        assert_ne!(current_source, source);
        shell.question_typed::<SessionPlace>(current_source, "current draft".into(), cx);
        let draft = shell
            .areas
            .conversation
            .answering
            .as_ref()
            .unwrap()
            .drafts
            .clone();
        shell.question_typed::<SessionPlace>(source, "stale text".into(), cx);
        assert_eq!(
            shell.areas.conversation.answering.as_ref().unwrap().drafts,
            draft
        );
        shell.question_entered::<SessionPlace>(source, "stale text".into(), window, cx);
        assert!(
            shell
                .areas
                .conversation
                .answering
                .as_ref()
                .unwrap()
                .sending
                .is_none()
        );
        assert!(!shell.question_cursor::<SessionPlace>(cursor));
        // Prove the successor can actually submit: ignoring the old source
        // guard above would set this flag and fail the preceding assertion.
        shell.question_entered::<SessionPlace>(current_source, "current draft".into(), window, cx);
        assert_eq!(
            shell.areas.conversation.answering.as_ref().unwrap().sending,
            Some(ResponseAction::Submit)
        );
    });
}

#[gpui::test]
fn dismiss_progress_belongs_to_the_clicked_action_even_when_the_answer_is_accepted(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        prepare(shell, false, window, cx);
        let id = shell.first_question::<SessionPlace>().unwrap();
        let question = waiting_at::<SessionPlace>(shell, id).unwrap();
        let values = std::collections::BTreeMap::new();
        shell.question_send::<SessionPlace>(
            question.id,
            QuestionAnswer::Accept { values },
            ResponseAction::Dismiss,
            cx,
        );
        assert_eq!(
            shell.areas.conversation.answering.as_ref().unwrap().sending,
            Some(ResponseAction::Dismiss)
        );
    });
}

#[gpui::test]
fn an_uncertain_question_keeps_its_original_editor_and_cannot_resubmit(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        prepare(shell, false, window, cx);
        let form = shell.areas.conversation.answering.as_ref().unwrap();
        let id = form.id;
        let source = form.field.id();
        let drafts = form.drafts.clone();
        shell
            .areas
            .conversation
            .answering
            .get_mut(id)
            .unwrap()
            .sending = Some(ResponseAction::Submit);
        let request = vessel_app_view::responses::Request::Question(id);
        shell
            .areas
            .conversation
            .response_failures
            .failed(request, vessel_app_view::responses::Problem::Unconfirmed);
        assert!(!shell.question_editable::<SessionPlace>(id));
        shell.question_typed::<SessionPlace>(source, "late uncertain edit".into(), cx);
        shell.question_entered::<SessionPlace>(source, "late uncertain submit".into(), window, cx);
        let held = shell.areas.conversation.answering.get(id).unwrap();
        assert_eq!(held.field.id(), source);
        assert_eq!(held.drafts, drafts);
        assert_eq!(held.sending, Some(ResponseAction::Submit));
        // A correlated no-effect refusal is a different authority from a
        // snapshot still containing this request. It can permit correction.
        shell
            .areas
            .conversation
            .response_failures
            .failed(request, vessel_app_view::responses::Problem::Unsent);
        assert!(shell.question_editable::<SessionPlace>(id));
    });
}

#[gpui::test]
fn a_previous_step_source_cannot_type_or_submit_the_replacement_step(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        prepare(shell, false, window, cx);
        let form = shell.areas.conversation.answering.as_ref().unwrap();
        let id = form.id;
        let previous = form.field.id();
        shell.question_typed::<SessionPlace>(previous, "previous page draft".into(), cx);
        shell.question_show_at::<SessionPlace>(id, 2, window, cx);
        let current = shell
            .areas
            .conversation
            .answering
            .get(id)
            .unwrap()
            .field
            .id();
        assert_ne!(previous, current);
        shell.question_typed::<SessionPlace>(current, "current page draft".into(), cx);
        let drafts = shell
            .areas
            .conversation
            .answering
            .get(id)
            .unwrap()
            .drafts
            .clone();
        shell.question_typed::<SessionPlace>(previous, "stale prior-page change".into(), cx);
        shell.question_entered::<SessionPlace>(
            previous,
            "stale prior-page Enter".into(),
            window,
            cx,
        );
        let form = shell.areas.conversation.answering.get(id).unwrap();
        assert_eq!(form.step, 2);
        assert_eq!(form.drafts, drafts);
        assert_eq!(form.sending, None);
        shell.question_show_at::<SessionPlace>(id, 1, window, cx);
        assert_eq!(
            shell
                .areas
                .conversation
                .answering
                .get(id)
                .unwrap()
                .field
                .text(cx),
            "previous page draft"
        );
    });
}
