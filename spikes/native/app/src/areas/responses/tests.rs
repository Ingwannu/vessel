//! Same-session successors, cross-client cancellation and navigation races.
use super::*;
use crate::shell::language_tests;
use vessel_app_view::timeline::Live;
use vessel_app_view::{connection::ConnectionEpochs, devices::MachineKey};
use vessel_proto::{
    PermissionId, PermissionOption, PermissionOptionId, PermissionOptionKind, PermissionRequest,
    SessionId, ToolCallId,
};

fn permission() -> PermissionRequest {
    PermissionRequest {
        id: PermissionId::random(),
        tool_call: ToolCallId::new("tool"),
        options: vec![PermissionOption {
            id: PermissionOptionId::new("approve"),
            name: "Proceed".into(),
            kind: PermissionOptionKind::AllowOnce,
        }],
    }
}
fn begin(shell: &mut Shell) -> Pending {
    let node = vessel_proto::NodeId::random();
    let connection = ConnectionEpochs::default()
        .admit(MachineKey::from_book("scratch"), node)
        .unwrap();
    shell.fleet.machines = vec![vessel_app_view::Machine {
        name: "scratch".into(),
        node: Some(node),
        connection_epoch: Some(connection.epoch()),
        reach: vessel_app_view::Reach::Online,
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol: Some(vessel_proto::PROTOCOL_VERSION),
        version: None,
    }];
    let session = SessionId::random();
    let permission = permission();
    let area = &mut shell.areas.conversation;
    area.open("scratch", session);
    area.live = Live {
        permissions: vec![permission.clone()],
        questions: Vec::new(),
    };
    let _previous = area
        .permit_sending
        .insert(permission.id, permission.options[0].id.clone());
    shell.response_at(
        Seat::SESSION,
        Answer::Permit(permission.id, permission.options[0].id.clone()),
    );
    // Obtain the original correlation without a second submission. The
    // registry deliberately refuses an identical request until it settles.
    let request = Request::Permission(permission.id);
    assert_eq!(shell.areas.responses.seats.len(), 1);
    let ticket = *shell.areas.responses.seats.keys().next().unwrap();
    Pending {
        ticket,
        target: Target {
            connection,
            session,
        },
        request,
    }
}

#[gpui::test]
fn a_late_refusal_cannot_decorate_a_successor_canceled_or_other_target(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, _cx| {
        for successor in [false, true] {
            let pending = begin(shell);
            shell.areas.conversation.live.permissions = if successor {
                vec![permission()]
            } else {
                Vec::new()
            };
            shell.areas.conversation.refused = None;
            shell.response_answered(&Reply {
                pending: pending.clone(),
                result: Err(Problem::Unconfirmed),
            });
            assert_eq!(
                shell
                    .areas
                    .conversation
                    .response_failures
                    .get(pending.request),
                None
            );
            assert!(shell.areas.responses.seats.is_empty());
        }
        let pending = begin(shell);
        shell
            .areas
            .conversation
            .open("other", pending.target.session);
        shell.response_answered(&Reply {
            pending: pending.clone(),
            result: Err(Problem::Unsent),
        });
        assert_eq!(
            shell
                .areas
                .conversation
                .response_failures
                .get(pending.request),
            None
        );
    });
}

#[gpui::test]
fn an_uncertain_result_preserves_the_card_and_cannot_be_reopened_by_stale_reads(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, _cx| {
        let pending = begin(shell);
        shell.response_answered(&Reply {
            pending: pending.clone(),
            result: Err(Problem::Unconfirmed),
        });
        assert_eq!(
            shell
                .areas
                .conversation
                .response_failures
                .get(pending.request),
            Some(Problem::Unconfirmed)
        );
        // A failed refresh must not replace the retained uncertainty.
        // An ordinary Pending snapshot is not proof that no effect happened.
        let _followup = shell
            .areas
            .conversation
            .answered(vessel_app_link::ActAnswer {
                terminal: None,
                machine: "scratch".into(),
                session: pending.target.session,
                kind: vessel_app_link::ActKind::Conversation,
                result: Err(vessel_app_link::ActFailed {
                    code: Some(vessel_proto::ErrorCode::Unavailable),
                    message: "refresh failed".into(),
                }),
            });
        assert_eq!(
            shell
                .areas
                .conversation
                .response_failures
                .get(pending.request),
            Some(Problem::Unconfirmed)
        );
        assert!(shell.areas.conversation.response_waiting(pending.request));
        let permission = shell.areas.conversation.live.permissions[0].clone();
        let stale_live = shell.areas.conversation.live.clone();
        shell.areas.conversation.live = stale_live;
        shell.areas.conversation.reconcile_responses();
        assert!(
            !shell
                .areas
                .conversation
                .response_failures
                .retryable(pending.request)
        );
        shell.response_at(
            Seat::SESSION,
            Answer::Permit(permission.id, permission.options[0].id.clone()),
        );
        assert!(shell.areas.responses.seats.is_empty());
        shell.response_answered(&Reply {
            pending: pending.clone(),
            result: Ok(()),
        });
        assert!(shell.areas.responses.seats.is_empty());
        assert_eq!(
            shell
                .areas
                .conversation
                .response_failures
                .get(pending.request),
            Some(Problem::Unconfirmed)
        );
    });
}

#[gpui::test]
fn an_offline_cached_request_is_retained_without_admission_or_replay(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _, _| {
        let pending = begin(shell);
        shell.response_answered(&Reply {
            pending: pending.clone(),
            result: Err(Problem::Unsent),
        });
        let permission = shell.areas.conversation.live.permissions[0].clone();
        shell.fleet.machines[0].reach = vessel_app_view::Reach::Unreachable("offline".into());
        assert!(!shell.areas.conversation.response_online(&shell.fleet));
        shell.response_at(
            Seat::SESSION,
            Answer::Permit(permission.id, permission.options[0].id.clone()),
        );
        assert!(shell.areas.responses.seats.is_empty());
        assert!(shell.areas.conversation.response_waiting(pending.request));
        shell.fleet.machines[0].reach = vessel_app_view::Reach::Online;
        assert!(shell.areas.conversation.response_online(&shell.fleet));
        assert!(shell.areas.responses.seats.is_empty());
        shell.areas.conversation.lost = Some("stream closed".into());
        assert!(!shell.areas.conversation.response_online(&shell.fleet));
    });
}

#[gpui::test]
fn unoffered_options_and_invalid_question_values_do_not_admit_network_work(
    cx: &mut gpui::TestAppContext,
) {
    use std::collections::BTreeMap;
    use vessel_proto::conversation::{AnswerValue, FieldKind, QuestionField};
    use vessel_proto::{Question, QuestionAnswer, QuestionId};
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _, _| {
        let pending = begin(shell);
        shell.response_answered(&Reply {
            pending: pending.clone(),
            result: Err(Problem::Unsent),
        });
        let Request::Permission(permission) = pending.request else {
            panic!("permission fixture")
        };
        shell.response_at(
            Seat::SESSION,
            Answer::Permit(permission, PermissionOptionId::new("not-offered")),
        );
        assert!(shell.areas.responses.seats.is_empty());
        assert_eq!(
            shell
                .areas
                .conversation
                .response_failures
                .get(pending.request),
            Some(Problem::Invalid)
        );
        let question = Question {
            id: QuestionId::random(),
            message: "A Boolean question".into(),
            tool_call: None,
            fields: vec![QuestionField {
                name: "answer".parse().unwrap(),
                title: None,
                description: None,
                required: true,
                kind: FieldKind::Boolean { default: None },
            }],
        };
        let id = question.id;
        shell.areas.conversation.live.questions = vec![question];
        shell.response_at(
            Seat::SESSION,
            Answer::Question(
                id,
                QuestionAnswer::Accept {
                    values: BTreeMap::from([(
                        "answer".parse().unwrap(),
                        AnswerValue::Text("not Boolean".into()),
                    )]),
                },
            ),
        );
        assert!(shell.areas.responses.seats.is_empty());
        assert_eq!(
            shell
                .areas
                .conversation
                .response_failures
                .get(Request::Question(id)),
            Some(Problem::Invalid)
        );
    });
}
