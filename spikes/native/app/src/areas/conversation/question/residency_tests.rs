//! Real pending form resources behind the sole staged registry, without I/O.
use super::*;
use crate::areas::layout::residents::SessionResources;
use crate::areas::session::SessionView;
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_view::layout::{
    AgentRole, Layout, LayoutKey, LayoutMode, OpenIntent, PaneId, Placement, Target,
};
use vessel_app_view::residents::{
    HoldReason, OperationKind, OwnerReadiness, PanelClaim, ResidentProblem, ResidentToken,
    ResourceOwner, SessionAddress, Visibility,
};
use vessel_app_view::responses::{Problem as ResponseProblem, Request};
use vessel_proto::conversation::{FieldKind, QuestionField};
use vessel_proto::{NodeId, SessionId, WorkspaceId};

struct Fixture {
    token: ResidentToken,
    claim: PanelClaim,
    question: QuestionId,
}
fn form(id: QuestionId, window: &mut Window, cx: &mut Context<'_, Shell>) -> (Question, Answering) {
    let question = Question {
        id,
        message: "Retain this question".into(),
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
    };
    let field = TextField::with_source(
        "Answer",
        Shell::question_entered::<SessionPlace>,
        Shell::question_typed::<SessionPlace>,
        window,
        cx,
    );
    field.set_text("retained original input", window, cx);
    let mut drafts = empty_drafts(&question);
    let _previous = drafts.insert(
        "answer".parse().unwrap(),
        Draft::Text("retained original input".into()),
    );
    let answering = Answering {
        id,
        drafts,
        step: 0,
        field,
        sending: Some(ResponseAction::Submit),
        validation: None,
    };
    (question, answering)
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
    let token = shell
        .areas
        .session_residents
        .claim_session(
            SessionAddress { node, session },
            &claim,
            PaneId::MAIN,
            |_| {
                let mut conversation = ConversationArea::new(cx);
                conversation.open("original", session);
                let (question, answering) = form(id, window, cx);
                conversation.live.questions = vec![question];
                conversation.answering.insert(answering);
                SessionResources {
                    session: SessionView::new(window, cx),
                    conversation,
                }
            },
        )
        .unwrap();
    shell
        .areas
        .session_residents
        .present(token, &claim, PaneId::MAIN, Visibility::Visible)
        .unwrap();
    Fixture {
        token,
        claim,
        question: id,
    }
}

#[gpui::test]
fn retiring_an_original_request_retains_the_actual_form_and_blocks_resend(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let fixture = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let request = Request::Question(fixture.question);
        let context = residents.context(fixture.token, &fixture.claim).unwrap();
        let source = residents
            .resources(&context)
            .unwrap()
            .conversation
            .answering
            .get(fixture.question)
            .unwrap()
            .field
            .id();
        let old = residents
            .begin_follow(fixture.token, |resources, notice| {
                resources.conversation.requests_retired(notice)
            })
            .unwrap();
        let ticket = residents.request_admit(old, request).unwrap();
        assert_eq!(
            residents
                .resources(&context)
                .unwrap()
                .conversation
                .requests_readiness(),
            OwnerReadiness::Hold(HoldReason::PendingEffect)
        );
        residents
            .present(
                fixture.token,
                &fixture.claim,
                PaneId::MAIN,
                Visibility::RetainedHidden,
            )
            .unwrap();
        let next = residents
            .begin_follow(fixture.token, |resources, notice| {
                resources.conversation.requests_retired(notice)
            })
            .unwrap();
        let context = residents.context(fixture.token, &fixture.claim).unwrap();
        let area = &residents.resources(&context).unwrap().conversation;
        let retained = area.answering.get(fixture.question).unwrap();
        assert_eq!(retained.field.id(), source);
        assert_eq!(retained.field.text(cx), "retained original input");
        assert_eq!(
            retained.drafts.get(&"answer".parse().unwrap()),
            Some(&Draft::Text("retained original input".into()))
        );
        assert_eq!(
            area.response_failures.get(request),
            Some(ResponseProblem::Unconfirmed)
        );
        assert_eq!(
            area.requests_readiness(),
            OwnerReadiness::Hold(HoldReason::UncertainEffect)
        );
        assert_eq!(
            residents.request_admit(next, request),
            Err(ResidentProblem::Held)
        );
        assert_eq!(
            residents.request_settle::<()>(old, ticket, |_, _| panic!("late reply installed")),
            Err(ResidentProblem::StaleBinding)
        );
        // Make Requests the sole reason eviction is refused.
        for owner in ResourceOwner::ALL {
            residents
                .readiness(fixture.token, owner, OwnerReadiness::Ready)
                .unwrap();
        }
        residents
            .readiness(
                fixture.token,
                ResourceOwner::Requests,
                OwnerReadiness::Hold(HoldReason::UncertainEffect),
            )
            .unwrap();
        assert!(matches!(
            residents.retire_clean(fixture.token),
            Err(ResidentProblem::Held)
        ));
    });
}

#[gpui::test]
fn settlement_rejects_foreign_and_duplicate_tickets_before_owner_mutation(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let fixture = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let binding = residents
            .begin_follow(fixture.token, |resources, notice| {
                resources.conversation.requests_retired(notice)
            })
            .unwrap();
        let foreign = residents
            .admit_operation(binding, OperationKind::Request)
            .unwrap();
        assert_eq!(
            residents
                .request_settle::<()>(binding, foreign, |_, _| panic!("foreign ticket installed")),
            Err(ResidentProblem::UnadmittedOperation)
        );
        // Rejection must not consume another owner's canonical admission.
        residents
            .settle(binding, OperationKind::Request, foreign, |_| ())
            .unwrap();
        let request = Request::Question(fixture.question);
        let ticket = residents.request_admit(binding, request).unwrap();
        assert_eq!(
            residents
                .request_settle(binding, ticket, |_, original| original)
                .unwrap(),
            request
        );
        assert_eq!(
            residents.request_settle::<()>(binding, ticket, |_, _| panic!("duplicate installed")),
            Err(ResidentProblem::UnadmittedOperation)
        );
        let context = residents.context(fixture.token, &fixture.claim).unwrap();
        // Settlement alone never fabricates a resolution or drops a form.
        assert_eq!(
            residents
                .resources(&context)
                .unwrap()
                .conversation
                .requests_readiness(),
            OwnerReadiness::Hold(HoldReason::DirtyInput)
        );
    });
}

#[gpui::test]
fn a_resolved_log_request_does_not_hold_its_stale_snapshot_form_forever(cx: &mut TestAppContext) {
    use vessel_proto::{AgentEvent, EventRecord, EventSeq, QuestionOutcome, SessionEvent};
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let fixture = fixture(shell, window, cx);
        let residents = &mut shell.areas.session_residents;
        let context = residents.context(fixture.token, &fixture.claim).unwrap();
        let area = &mut residents.resources_mut(&context).unwrap().conversation;
        let question = area.live.questions[0].clone();
        let records = [
            EventRecord {
                seq: EventSeq::ZERO,
                at: chrono::DateTime::UNIX_EPOCH,
                event: SessionEvent::Agent {
                    event: AgentEvent::QuestionAsked(question),
                },
            },
            EventRecord {
                seq: EventSeq::new(1),
                at: chrono::DateTime::UNIX_EPOCH,
                event: SessionEvent::Agent {
                    event: AgentEvent::QuestionResolved {
                        id: fixture.question,
                        outcome: QuestionOutcome::Cancelled,
                    },
                },
            },
        ];
        assert!(area.take(&fixture.token.address.session, &records));
        assert_eq!(
            area.live.questions.len(),
            1,
            "the older snapshot is retained as read"
        );
        assert!(
            area.answering.get(fixture.question).is_some(),
            "the local field still exists"
        );
        assert!(!area.response_waiting(Request::Question(fixture.question)));
        assert_eq!(area.requests_readiness(), OwnerReadiness::Ready);
    });
}
