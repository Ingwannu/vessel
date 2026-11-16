//! Actual retained editor + sole staged registry; no product bundle installation.
use super::super::requests::{Form, Requests, Stage};
use super::*;
use crate::areas::layout::residents::{SessionResidents, SessionResources};
use crate::areas::session::SessionView;
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_kit::field::FieldId;
use vessel_app_view::connection::{ConnectionContext, ConnectionEpochs, MachineKey};
use vessel_app_view::layout::{
    AgentRole, Layout, LayoutKey, LayoutMode, OpenIntent, PaneId, Placement, Target,
};
use vessel_app_view::requests::Request;
use vessel_app_view::residents::{
    FollowBinding, HoldReason, OperationKind, OwnerReadiness, PanelClaim, ResidentProblem,
    ResidentToken, SessionAddress, Visibility,
};
use vessel_proto::conversation::{FieldKind, QuestionField};
use vessel_proto::{
    AgentEvent, EventRecord, EventSeq, NodeId, PermissionId, PermissionOption, PermissionOptionId,
    PermissionOptionKind, PermissionRequest, QuestionOutcome, SessionEvent, SessionId, ToolCallId,
    WorkspaceId,
};

struct Fixture {
    connection: ConnectionContext,
    token: ResidentToken,
    claim: PanelClaim,
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
pub(super) fn entered(
    _: &mut Shell,
    _: FieldId,
    _: String,
    _: &mut Window,
    _: &mut Context<'_, Shell>,
) {
}
pub(super) fn changed(_: &mut Shell, _: FieldId, _: String, _: &mut Context<'_, Shell>) {}
pub(super) fn question(id: QuestionId) -> Question {
    Question {
        id,
        message: "Retained form".into(),
        tool_call: None,
        fields: vec![QuestionField {
            name: "answer".parse().unwrap(),
            title: None,
            description: None,
            required: true,
            kind: FieldKind::Text {
                min_length: None,
                max_length: None,
                format: None,
                pattern: None,
                default: None,
            },
        }],
    }
}
fn fixture(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) -> Fixture {
    let node = NodeId::random();
    let session = SessionId::random();
    let mut layout = Layout::default();
    let claim = PanelClaim {
        workspace: LayoutKey::Workspace {
            node,
            workspace: WorkspaceId::random(),
        },
        tab: layout
            .open(
                Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )
            .unwrap(),
    };
    let id = QuestionId::random();
    let field = TextField::with_source("Answer", entered, changed, window, cx);
    field.set_text("retained original input", window, cx);
    let source = field.id();
    let token = shell
        .areas
        .session_residents
        .claim_session(
            SessionAddress { node, session },
            &claim,
            PaneId::MAIN,
            |context| {
                assert_eq!(context.resident.address.session, session);
                let mut conversation = ConversationArea::new(cx);
                let question = question(id);
                let mut drafts = starting_drafts(&question);
                let _previous = drafts.insert(
                    "answer".parse().unwrap(),
                    Draft::Text("retained original input".into()),
                );
                conversation.live.questions = vec![question];
                conversation.answering = Some(Answering {
                    id,
                    drafts,
                    step: 0,
                    field,
                    sent: false,
                });
                assert!(
                    conversation.shown.is_none(),
                    "Shown is not source authority"
                );
                SessionResources::new(SessionView::new(window, cx), conversation)
            },
        )
        .unwrap();
    shell
        .areas
        .session_residents
        .present(token, &claim, PaneId::MAIN, Visibility::Visible)
        .unwrap();
    shell.areas.session_residents.focus(token, &claim).unwrap();
    Fixture {
        connection: ConnectionEpochs::default()
            .admit(MachineKey::from_book("original"), node)
            .unwrap(),
        token,
        claim,
        question: id,
        source,
    }
}
fn admit(
    owner: &mut Requests,
    residents: &mut SessionResidents,
    fixture: &Fixture,
    binding: FollowBinding,
) -> vessel_app_view::residents::OperationTicket {
    residents
        .requests_test_admit(owner, &fixture.connection, binding, fixture.form())
        .unwrap()
}
fn record(id: QuestionId) -> EventRecord {
    EventRecord {
        seq: EventSeq::ZERO,
        at: chrono::DateTime::UNIX_EPOCH,
        event: SessionEvent::Agent {
            event: AgentEvent::QuestionResolved {
                id,
                outcome: QuestionOutcome::Cancelled,
            },
        },
    }
}

#[gpui::test]
fn origin_refusal_mints_no_ticket_and_dropped_preparation_keeps_form(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let f = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let binding = residents.begin_follow(f.token, |_, _| {}).unwrap();
        let context = residents.context(f.token, &f.claim).unwrap();
        let mut owner = Requests::default();
        let wrong = ConnectionEpochs::default()
            .admit(MachineKey::from_book("original"), NodeId::random())
            .unwrap();
        assert_eq!(
            residents.requests_test_admit(&mut owner, &wrong, binding, f.form()),
            Err(ResidentProblem::OwnerConflict)
        );
        let area = &mut residents.resources_mut(&context).unwrap().conversation;
        {
            let _prepared = owner
                .prepare(&f.connection, binding, f.form(), area)
                .unwrap();
        }
        let form = area.answering.as_ref().unwrap();
        assert!(!form.sent);
        assert_eq!(form.field.id(), f.source);
        assert_eq!(form.field.text(cx), "retained original input");
        assert_eq!(
            owner.readiness(area),
            OwnerReadiness::Hold(HoldReason::DirtyInput)
        );
        residents
            .retire_follow(binding, |_, notice| assert!(notice.operations().is_empty()))
            .unwrap();
        assert_eq!(
            residents.requests_test_admit(&mut owner, &f.connection, binding, f.form()),
            Err(ResidentProblem::StaleBinding)
        );
        let area = &residents.resources(&context).unwrap().conversation;
        assert!(!area.answering.as_ref().unwrap().sent);
        assert_eq!(
            area.answering.as_ref().unwrap().field.text(cx),
            "retained original input"
        );
    });
}

#[gpui::test]
fn retirement_keeps_actual_field_and_exact_resolution_releases_only_its_hold(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let f = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let old = residents.begin_follow(f.token, |_, _| {}).unwrap();
        let mut owner = Requests::default();
        let ticket = admit(&mut owner, residents, &f, old);
        let context = residents.context(f.token, &f.claim).unwrap();
        assert_eq!(
            owner.readiness(&residents.resources(&context).unwrap().conversation),
            OwnerReadiness::Hold(HoldReason::PendingEffect)
        );
        owner.observed(old, &[record(f.question)]);
        assert_eq!(
            owner.stage(f.request()),
            Some(Stage::Sending),
            "active ticket still requires canonical settle"
        );
        let fresh = residents
            .begin_follow(f.token, |_, notice| owner.retired(notice))
            .unwrap();
        let area = &mut residents.resources_mut(&context).unwrap().conversation;
        assert_eq!(
            owner.readiness(area),
            OwnerReadiness::Hold(HoldReason::UncertainEffect)
        );
        assert!(matches!(
            owner.prepare(&f.connection, fresh, f.form(), area),
            Err(ResidentProblem::Held)
        ));
        assert_eq!(
            owner.settling(old, ticket),
            Err(ResidentProblem::UnadmittedOperation)
        );
        residents
            .present(f.token, &f.claim, PaneId::MAIN, Visibility::RetainedHidden)
            .unwrap();
        residents
            .receive(fresh, |resources| {
                let area = &resources.conversation;
                let form = area.answering.as_ref().unwrap();
                assert_eq!(form.field.id(), f.source);
                assert_eq!(form.field.text(cx), "retained original input");
                assert_eq!(
                    form.drafts.get(&"answer".parse().unwrap()),
                    Some(&Draft::Text("retained original input".into()))
                );
                owner.observed(fresh, &[record(QuestionId::random())]);
                assert_eq!(owner.stage(f.request()), Some(Stage::Uncertain));
                owner.observed(fresh, &[record(f.question)]);
                assert_eq!(owner.stage(f.request()), None);
                assert_eq!(
                    form.field.id(),
                    f.source,
                    "resolution hook retains original engine"
                );
            })
            .unwrap();
    });
}

#[gpui::test]
fn stale_editor_and_foreign_or_duplicate_tickets_cannot_settle(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let f = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let binding = residents.begin_follow(f.token, |_, _| {}).unwrap();
        let mut owner = Requests::default();
        let other = TextField::with_source("Other", entered, changed, window, cx);
        let context = residents.context(f.token, &f.claim).unwrap();
        let area = &mut residents.resources_mut(&context).unwrap().conversation;
        assert!(matches!(
            owner.prepare(
                &f.connection,
                binding,
                Form::Question {
                    id: f.question,
                    source: other.id()
                },
                area
            ),
            Err(ResidentProblem::StaleEpoch)
        ));
        let foreign = residents
            .admit_operation(binding, OperationKind::Composer)
            .unwrap();
        assert_eq!(
            owner.settling(binding, foreign),
            Err(ResidentProblem::UnadmittedOperation)
        );
        let ticket = admit(&mut owner, residents, &f, binding);
        assert_eq!(owner.settling(binding, ticket), Ok(f.request()));
        residents
            .settle(binding, OperationKind::Request, ticket, |_| {
                assert_eq!(owner.settled(binding, ticket), Ok(f.request()));
            })
            .unwrap();
        assert_eq!(
            owner.settling(binding, ticket),
            Err(ResidentProblem::UnadmittedOperation)
        );
        assert!(
            residents
                .settle(binding, OperationKind::Request, ticket, |_| {})
                .is_err()
        );
    });
}

#[gpui::test]
fn permission_preparation_accepts_only_an_actual_offered_option(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let f = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let binding = residents.begin_follow(f.token, |_, _| {}).unwrap();
        let context = residents.context(f.token, &f.claim).unwrap();
        let area = &mut residents.resources_mut(&context).unwrap().conversation;
        let id = PermissionId::random();
        let offered: PermissionOptionId = PermissionOptionId::new("provider-actual-id");
        area.live.permissions.push(PermissionRequest {
            id,
            tool_call: ToolCallId::new("actual-tool"),
            options: vec![PermissionOption {
                id: offered.clone(),
                name: "Allow once".into(),
                kind: PermissionOptionKind::AllowOnce,
            }],
        });
        let mut owner = Requests::default();
        assert!(matches!(
            owner.prepare(
                &f.connection,
                binding,
                Form::Permission {
                    id,
                    option: PermissionOptionId::new("invented")
                },
                area
            ),
            Err(ResidentProblem::UnadmittedOperation)
        ));
        {
            let _prepared = owner
                .prepare(
                    &f.connection,
                    binding,
                    Form::Permission {
                        id,
                        option: offered.clone(),
                    },
                    area,
                )
                .unwrap();
        }
        assert!(area.permit_sending.is_none());
        let _ticket = residents
            .requests_test_admit(
                &mut owner,
                &f.connection,
                binding,
                Form::Permission {
                    id,
                    option: offered.clone(),
                },
            )
            .unwrap();
        let area = &residents.resources(&context).unwrap().conversation;
        assert_eq!(area.permit_sending.as_ref(), Some(&(id, offered)));
        assert_eq!(owner.stage(Request::Permission(id)), Some(Stage::Sending));
    });
}
