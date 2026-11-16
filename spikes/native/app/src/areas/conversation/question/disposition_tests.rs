//! Logical dispositions use the installed owner and actual minter/ticket table.
//! No test fabricates a delivery guard or proves a transport never dispatched.
use super::requests_tests::{changed, entered, question};
use super::*;
use crate::areas::conversation::requests::{Disposition, Form, Stage, Transition};
use crate::areas::layout::residents::{
    SessionAdmission, SessionOpening, SessionResidents, SessionResources,
};
use crate::areas::session::SessionView;
use crate::shell::language_tests;
use gpui::TestAppContext;
use std::cell::Cell;
use vessel_app_kit::field::FieldId;
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::layout::{LookupProblem, WorkspaceLocation, WorkspaceLookup};
use vessel_app_view::requests::Request;
use vessel_app_view::residents::{
    HoldReason, OperationKind, OperationTicket, OwnerReadiness, ResidentProblem,
};
use vessel_proto::{
    NodeId, PermissionId, PermissionOption, PermissionOptionId, PermissionOptionKind,
    PermissionRequest, SessionId, ToolCallId, WorkspaceId,
};

struct Registry {
    node: NodeId,
    session: SessionId,
    workspace: WorkspaceId,
}
impl WorkspaceLookup for Registry {
    fn session_workspace(
        &self,
        node: NodeId,
        session: SessionId,
    ) -> Result<Option<WorkspaceId>, LookupProblem> {
        Ok((node == self.node && session == self.session).then_some(self.workspace))
    }
    fn workspace_location(
        &self,
        _: NodeId,
        _: WorkspaceId,
    ) -> Result<WorkspaceLocation, LookupProblem> {
        Err(LookupProblem::Unavailable)
    }
}
struct Fixture {
    admission: SessionAdmission,
    question: QuestionId,
    source: FieldId,
}
impl Fixture {
    fn form(&self) -> Form {
        Form::Question {
            id: self.question,
            source: self.source,
        }
    }
    fn request(&self) -> Request {
        Request::Question(self.question)
    }
}
fn fixture(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) -> Fixture {
    let node = NodeId::random();
    let registry = Registry {
        node,
        session: SessionId::random(),
        workspace: WorkspaceId::random(),
    };
    let original = ConnectionEpochs::default()
        .admit(MachineKey::from_book("fixture"), node)
        .unwrap();
    let opening = SessionOpening::resolve(&registry, original, registry.session).unwrap();
    let id = QuestionId::random();
    let source = Cell::new(None);
    let admission = shell
        .admit_session_open(
            &opening,
            &registry,
            |context, window, cx| {
                assert_eq!(context.resident.address.node, node);
                let field = TextField::with_source("Answer", entered, changed, window, cx);
                field.set_text("retained original input", window, cx);
                source.set(Some(field.id()));
                let question = question(id);
                let mut drafts = starting_drafts(&question);
                let _old = drafts.insert(
                    "answer".parse().unwrap(),
                    Draft::Text("retained original input".into()),
                );
                let mut area = ConversationArea::new(cx);
                area.live.questions = vec![question];
                area.answering = Some(Answering {
                    id,
                    drafts,
                    step: 0,
                    field,
                    sent: false,
                });
                assert!(area.shown.is_none());
                SessionResources::new(SessionView::new(window, cx), area)
            },
            |_, _| {},
            window,
            cx,
        )
        .unwrap();
    Fixture {
        admission,
        question: id,
        source: source.get().unwrap(),
    }
}
fn admit(residents: &mut SessionResidents, f: &Fixture, form: Form) -> OperationTicket {
    residents
        .admit_request(
            &f.admission.connection,
            &f.admission.context,
            f.admission.binding,
            form,
        )
        .unwrap()
}
// Mirrors only the trusted owner's order, NOT its missing never-dispatched proof:
// exact owner correlation -> consume the sole ticket -> owner-local transition.
fn fail(
    residents: &mut SessionResidents,
    f: &Fixture,
    ticket: OperationTicket,
    disposition: Disposition,
) -> Result<Transition, ResidentProblem> {
    let binding = f.admission.binding;
    let _request = residents.receive(binding, |r| r.requests.settling(binding, ticket))??;
    residents.settle(binding, OperationKind::Request, ticket, |r| {
        r.requests
            .failed(binding, ticket, disposition, &mut r.conversation)
    })?
}
fn permission(
    resources: &mut SessionResources,
) -> (PermissionId, PermissionOptionId, PermissionOptionId) {
    let id = PermissionId::random();
    let allow = PermissionOptionId::new("provider-allow");
    let deny = PermissionOptionId::new("provider-deny");
    resources
        .conversation
        .live
        .permissions
        .push(PermissionRequest {
            id,
            tool_call: ToolCallId::new("actual-tool"),
            options: vec![
                PermissionOption {
                    id: allow.clone(),
                    name: "Allow".into(),
                    kind: PermissionOptionKind::AllowOnce,
                },
                PermissionOption {
                    id: deny.clone(),
                    name: "Deny".into(),
                    kind: PermissionOptionKind::RejectOnce,
                },
            ],
        });
    (id, allow, deny)
}
fn retained(resources: &SessionResources, f: &Fixture, sent: bool, cx: &Context<'_, Shell>) {
    let form = resources.conversation.answering.as_ref().unwrap();
    assert_eq!(form.id, f.question);
    assert_eq!(form.field.id(), f.source);
    assert_eq!(form.sent, sent);
    assert_eq!(form.field.text(cx), "retained original input");
    assert_eq!(
        form.drafts.get(&"answer".parse().unwrap()),
        Some(&Draft::Text("retained original input".into()))
    );
}

#[gpui::test]
fn no_send_releases_only_the_original_form_and_a_late_duplicate_cannot_clear_retry(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let f = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let ticket = admit(residents, &f, f.form());
        assert_eq!(
            fail(residents, &f, ticket, Disposition::NoSend),
            Ok(Transition::Released(f.form()))
        );
        let r = residents.resources(&f.admission.context).unwrap();
        retained(r, &f, false, cx);
        assert_eq!(r.requests.stage(f.request()), None);
        assert_eq!(
            r.requests.readiness(&r.conversation),
            OwnerReadiness::Hold(HoldReason::DirtyInput)
        );
        let retry = admit(residents, &f, f.form());
        assert_ne!(retry, ticket, "a retry is a new explicit operation");
        assert!(fail(residents, &f, ticket, Disposition::NoSend).is_err());
        retained(
            residents.resources(&f.admission.context).unwrap(),
            &f,
            true,
            cx,
        );
        assert_eq!(
            fail(residents, &f, retry, Disposition::NoSend),
            Ok(Transition::Released(f.form()))
        );
        residents
            .retire_follow(f.admission.binding, |r, notice| {
                assert!(
                    notice.operations().is_empty(),
                    "both original tickets consumed once"
                );
                retained(r, &f, false, cx);
            })
            .unwrap();
    });
}

#[gpui::test]
fn a_started_uncertain_failure_retains_full_form_and_never_becomes_completion(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let f = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let ticket = admit(residents, &f, f.form());
        assert_eq!(
            fail(residents, &f, ticket, Disposition::Uncertain),
            Ok(Transition::Uncertain(f.form()))
        );
        let r = residents.resources(&f.admission.context).unwrap();
        retained(r, &f, true, cx);
        assert_eq!(r.requests.form(f.request()), Some(&f.form()));
        assert_eq!(r.requests.stage(f.request()), Some(Stage::Uncertain));
        assert_eq!(
            r.requests.readiness(&r.conversation),
            OwnerReadiness::Hold(HoldReason::UncertainEffect)
        );
        assert!(fail(residents, &f, ticket, Disposition::NoSend).is_err());
        let called = Cell::new(false);
        assert!(
            residents
                .settle_request(f.admission.binding, ticket, |_, _| called.set(true))
                .is_err()
        );
        assert!(!called.get());
        assert!(
            residents
                .admit_request(
                    &f.admission.connection,
                    &f.admission.context,
                    f.admission.binding,
                    f.form()
                )
                .is_err()
        );
        residents
            .retire_follow(f.admission.binding, |r, notice| {
                assert!(notice.operations().is_empty());
                assert_eq!(r.requests.form(f.request()), Some(&f.form()));
                assert_eq!(r.requests.stage(f.request()), Some(Stage::Uncertain));
                retained(r, &f, true, cx);
            })
            .unwrap();
    });
}

#[gpui::test]
fn no_send_cannot_release_a_replaced_editor_with_the_same_question_id(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let f = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let ticket = admit(residents, &f, f.form());
        let replacement = TextField::with_source("Replacement", entered, changed, window, cx);
        replacement.set_text("replacement input", window, cx);
        let replacement_id = replacement.id();
        let r = residents.resources_mut(&f.admission.context).unwrap();
        let form = r.conversation.answering.as_mut().unwrap();
        form.field = replacement;
        let _old = form.drafts.insert(
            "answer".parse().unwrap(),
            Draft::Text("replacement input".into()),
        );
        assert_ne!(replacement_id, f.source);
        assert_eq!(r.requests.form(f.request()), Some(&f.form()));
        assert_eq!(
            fail(residents, &f, ticket, Disposition::NoSend),
            Ok(Transition::Unchanged(f.form()))
        );
        let form = residents
            .resources(&f.admission.context)
            .unwrap()
            .conversation
            .answering
            .as_ref()
            .unwrap();
        assert_eq!(form.field.id(), replacement_id);
        assert!(form.sent);
        assert_eq!(form.field.text(cx), "replacement input");
        assert_eq!(
            form.drafts.get(&"answer".parse().unwrap()),
            Some(&Draft::Text("replacement input".into()))
        );
    });
}

#[gpui::test]
fn no_send_cannot_revive_a_withdrawn_form_even_if_its_editor_is_still_retained(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let f = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let ticket = admit(residents, &f, f.form());
        residents
            .resources_mut(&f.admission.context)
            .unwrap()
            .conversation
            .live
            .questions
            .clear();
        assert_eq!(
            fail(residents, &f, ticket, Disposition::NoSend),
            Ok(Transition::Unchanged(f.form()))
        );
        let r = residents.resources(&f.admission.context).unwrap();
        retained(r, &f, true, cx);
        assert!(r.conversation.live.questions.is_empty());
        assert_eq!(r.requests.stage(f.request()), None);
    });
}

#[gpui::test]
fn no_send_preserves_a_replacement_selected_permission_option(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let f = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let (id, allow, deny) = permission(residents.resources_mut(&f.admission.context).unwrap());
        let form = Form::Permission { id, option: allow };
        let ticket = admit(residents, &f, form.clone());
        residents
            .resources_mut(&f.admission.context)
            .unwrap()
            .conversation
            .permit_sending = Some((id, deny.clone()));
        assert_eq!(
            fail(residents, &f, ticket, Disposition::NoSend),
            Ok(Transition::Unchanged(form))
        );
        let r = residents.resources(&f.admission.context).unwrap();
        assert_eq!(r.conversation.permit_sending.as_ref(), Some(&(id, deny)));
        retained(r, &f, false, cx);
    });
}

#[gpui::test]
fn permission_no_send_does_not_clear_a_question_or_consume_a_foreign_ticket(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let f = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let question_ticket = admit(residents, &f, f.form());
        let (id, allow, _) = permission(residents.resources_mut(&f.admission.context).unwrap());
        let form = Form::Permission { id, option: allow };
        let ticket = admit(residents, &f, form.clone());
        let foreign = residents
            .admit_operation(f.admission.binding, OperationKind::Composer)
            .unwrap();
        assert!(fail(residents, &f, foreign, Disposition::NoSend).is_err());
        let r = residents.resources(&f.admission.context).unwrap();
        assert_eq!(
            r.requests.settling(f.admission.binding, question_ticket),
            Ok(f.request())
        );
        assert_eq!(r.requests.form(Request::Permission(id)), Some(&form));
        retained(r, &f, true, cx);
        assert_eq!(
            fail(residents, &f, ticket, Disposition::NoSend),
            Ok(Transition::Released(form))
        );
        residents
            .retire_follow(f.admission.binding, |r, notice| {
                assert_eq!(
                    notice.operations().len(),
                    2,
                    "question and foreign operation were untouched"
                );
                assert!(r.conversation.permit_sending.is_none());
                assert_eq!(r.requests.stage(f.request()), Some(Stage::Uncertain));
                retained(r, &f, true, cx);
            })
            .unwrap();
    });
}

#[gpui::test]
fn a_retired_operation_rejects_late_no_send_and_definitive_callbacks(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let f = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let ticket = admit(residents, &f, f.form());
        let fresh = residents
            .begin_follow(f.admission.context.resident, |r, notice| {
                assert_eq!(notice.operations().len(), 1);
                assert_eq!(r.requests.stage(f.request()), Some(Stage::Uncertain));
            })
            .unwrap();
        assert_ne!(fresh, f.admission.binding);
        assert!(fail(residents, &f, ticket, Disposition::NoSend).is_err());
        let called = Cell::new(false);
        assert!(
            residents
                .settle_request(f.admission.binding, ticket, |_, _| called.set(true))
                .is_err()
        );
        assert!(!called.get());
        residents
            .receive(fresh, |r| {
                assert_eq!(r.requests.form(f.request()), Some(&f.form()));
                assert_eq!(r.requests.stage(f.request()), Some(Stage::Uncertain));
                retained(r, &f, true, cx);
                assert_eq!(
                    r.requests.failed(
                        f.admission.binding,
                        ticket,
                        Disposition::NoSend,
                        &mut r.conversation
                    ),
                    Err(ResidentProblem::UnadmittedOperation)
                );
                retained(r, &f, true, cx);
            })
            .unwrap();
    });
}

#[gpui::test]
fn permission_uncertainty_retains_the_original_selected_option_across_local_replacement(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let f = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let (id, allow, deny) = permission(residents.resources_mut(&f.admission.context).unwrap());
        let form = Form::Permission {
            id,
            option: allow.clone(),
        };
        let ticket = admit(residents, &f, form.clone());
        assert_eq!(
            fail(residents, &f, ticket, Disposition::Uncertain),
            Ok(Transition::Uncertain(form.clone()))
        );
        let r = residents.resources_mut(&f.admission.context).unwrap();
        assert_eq!(r.requests.form(Request::Permission(id)), Some(&form));
        assert_eq!(r.conversation.permit_sending.as_ref(), Some(&(id, allow)));
        r.conversation.permit_sending = Some((id, deny.clone()));
        assert!(fail(residents, &f, ticket, Disposition::NoSend).is_err());
        let r = residents.resources(&f.admission.context).unwrap();
        assert_eq!(r.requests.form(Request::Permission(id)), Some(&form));
        assert_eq!(
            r.requests.stage(Request::Permission(id)),
            Some(Stage::Uncertain)
        );
        assert_eq!(r.conversation.permit_sending.as_ref(), Some(&(id, deny)));
        retained(r, &f, false, cx);
    });
}
