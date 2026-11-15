//! Same-session successors, cross-client cancellation and navigation races.
use super::*;
use crate::shell::language_tests;
use vessel_app_view::timeline::Live;
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
    shell.fleet.machines = vec![vessel_app_view::Machine {
        name: "scratch".into(),
        node: Some(vessel_proto::NodeId::random()),
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
    area.permit_sending = Some((permission.id, permission.options[0].id.clone()));
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
            machine: MachineKey::from_book("scratch"),
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
            shell.response_answered(Reply {
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
        shell.response_answered(Reply {
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
fn an_uncertain_result_preserves_the_card_and_duplicate_replies_cannot_clear_a_retry(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, _cx| {
        let pending = begin(shell);
        shell.response_answered(Reply {
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
        // A failed refresh used to overwrite the response's generic refusal,
        // leaving the original card permanently sending without a retry.
        let _followup = shell
            .areas
            .conversation
            .answered(vessel_app_link::ActAnswer {
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
        shell.response_at(
            Seat::SESSION,
            Answer::Permit(permission.id, permission.options[0].id.clone()),
        );
        assert_eq!(shell.areas.responses.seats.len(), 1);
        shell.response_answered(Reply {
            pending: pending.clone(),
            result: Ok(()),
        });
        assert_eq!(shell.areas.responses.seats.len(), 1);
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
fn an_offline_cached_request_is_retained_without_admission_or_replay(
    cx: &mut gpui::TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _, _| {
        let pending = begin(shell);
        shell.response_answered(Reply {
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
