//! Concrete Requests field and canonical atomic wrapper, without real wire guards.
use super::*;
use crate::areas::conversation::requests::{Form, Stage};
use vessel_app_view::requests::Request;
use vessel_proto::{
    AgentEvent, EventRecord, EventSeq, PermissionId, PermissionOption, PermissionOptionId,
    PermissionOptionKind, PermissionOutcome, PermissionRequest, SessionEvent, ToolCallId,
};

fn offered(residents: &mut SessionResidents, admission: &SessionAdmission) -> (PermissionId, Form) {
    let id = PermissionId::random();
    let option = PermissionOptionId::new("actual-option");
    residents
        .receive(admission.binding, |resources| {
            resources
                .conversation
                .live
                .permissions
                .push(PermissionRequest {
                    id,
                    tool_call: ToolCallId::new("actual-tool"),
                    options: vec![PermissionOption {
                        id: option.clone(),
                        name: "Allow once".into(),
                        kind: PermissionOptionKind::AllowOnce,
                    }],
                });
        })
        .unwrap();
    (id, Form::Permission { id, option })
}
fn resolved(id: PermissionId, seq: EventSeq) -> EventRecord {
    EventRecord {
        seq,
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: AgentEvent::PermissionResolved {
                id,
                outcome: PermissionOutcome::Cancelled,
            },
        },
    }
}

fn current_after_refusals(
    residents: &mut SessionResidents,
    admission: &SessionAdmission,
    form: &Form,
) -> vessel_app_view::residents::SessionContext {
    let context = &admission.context;
    let mut epochs = ConnectionEpochs::default();
    let _first = epochs
        .admit(
            admission.connection.machine().clone(),
            context.resident.address.node,
        )
        .unwrap();
    let successor = epochs
        .admit(
            admission.connection.machine().clone(),
            context.resident.address.node,
        )
        .unwrap();
    assert_ne!(successor, admission.connection);
    assert_eq!(
        residents.admit_request(&successor, context, admission.binding, form.clone()),
        Err(ResidentProblem::OwnerConflict)
    );
    residents
        .present(
            context.resident,
            &context.claim,
            context.pane,
            Visibility::RetainedHidden,
        )
        .unwrap();
    assert!(
        residents
            .admit_request(
                &admission.connection,
                context,
                admission.binding,
                form.clone()
            )
            .is_err()
    );
    let hidden = residents.context(context.resident, &context.claim).unwrap();
    assert_eq!(
        residents.admit_request(
            &admission.connection,
            &hidden,
            admission.binding,
            form.clone()
        ),
        Err(ResidentProblem::Held)
    );
    residents
        .present(
            context.resident,
            &context.claim,
            context.pane,
            Visibility::Visible,
        )
        .unwrap();
    residents.focus(context.resident, &context.claim).unwrap();
    residents.context(context.resident, &context.claim).unwrap()
}

#[gpui::test]
fn bundled_request_requires_current_input_original_connection_and_single_settlement(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let admission = admitted(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let (id, form) = offered(residents, &admission);
        let current = current_after_refusals(residents, &admission, &form);
        assert!(
            residents
                .resources(&current)
                .unwrap()
                .conversation
                .permit_sending
                .is_none()
        );
        let ticket = residents
            .admit_request(
                &admission.connection,
                &current,
                admission.binding,
                form.clone(),
            )
            .unwrap();
        assert_eq!(
            residents.admit_request(&admission.connection, &current, admission.binding, form),
            Err(ResidentProblem::Held)
        );
        assert_eq!(
            residents
                .resources(&current)
                .unwrap()
                .requests
                .stage(Request::Permission(id)),
            Some(Stage::Sending)
        );
        assert!(
            residents
                .settle(
                    admission.binding,
                    OperationKind::Timeline,
                    ticket,
                    |_| panic!("wrong namespace")
                )
                .is_err()
        );
        let mut callbacks = 0_usize;
        residents
            .settle_request(admission.binding, ticket, |resources, request| {
                callbacks = callbacks.saturating_add(1);
                assert_eq!(request, Request::Permission(id));
                resources.conversation.permit_sending = None;
            })
            .unwrap();
        assert!(
            residents
                .settle_request(admission.binding, ticket, |_, _| panic!("duplicate"))
                .is_err()
        );
        assert_eq!(callbacks, 1);
        assert_eq!(
            residents
                .resources(&current)
                .unwrap()
                .requests
                .stage(Request::Permission(id)),
            None
        );
    });
}

#[gpui::test]
fn bundled_request_retirement_and_scoped_records_preserve_only_exact_uncertainty(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let admission = admitted(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let (id, form) = offered(residents, &admission);
        let ticket = residents
            .admit_request(
                &admission.connection,
                &admission.context,
                admission.binding,
                form,
            )
            .unwrap();
        let fresh = residents
            .begin_follow(admission.context.resident, |resources, notice| {
                assert_eq!(notice.operations().len(), 1);
                assert_eq!(
                    resources.requests.stage(Request::Permission(id)),
                    Some(Stage::Uncertain)
                );
            })
            .unwrap();
        assert!(
            residents
                .settle_request(admission.binding, ticket, |_, _| panic!("retired"))
                .is_err()
        );
        let unrelated = resolved(PermissionId::random(), EventSeq::ZERO);
        let _unrelated_projection = shell
            .conversation_records_scoped(fresh, &[unrelated])
            .unwrap();
        let resources = shell
            .areas
            .session_residents
            .resources(&admission.context)
            .unwrap();
        assert_eq!(
            resources.requests.stage(Request::Permission(id)),
            Some(Stage::Uncertain)
        );
        assert!(resources.conversation.permit_sending.is_some());
        let exact = resolved(id, EventSeq::ZERO.next().unwrap());
        assert!(
            shell
                .conversation_records_scoped(admission.binding, std::slice::from_ref(&exact))
                .is_err()
        );
        let _resolved_projection = shell.conversation_records_scoped(fresh, &[exact]).unwrap();
        let resources = shell
            .areas
            .session_residents
            .resources(&admission.context)
            .unwrap();
        assert_eq!(resources.requests.stage(Request::Permission(id)), None);
    });
}
