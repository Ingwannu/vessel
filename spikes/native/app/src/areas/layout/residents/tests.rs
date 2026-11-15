//! Real retained field/timeline resources under one admission table, without I/O.
use super::*;
use crate::shell::{Shell, language_tests};
use gpui::TestAppContext;
use std::cell::Cell;
use std::time::Duration;
use vessel_app_view::layout::{
    AgentRole, Layout, LayoutKey, LayoutMode, OpenIntent, Placement, Position, Target,
};
use vessel_app_view::residents::{
    FollowBinding, OperationKind, OwnerReadiness, ResourceOwner, StopAction, StopCompletion,
    StopSettlement, StopWindowReceipt, Visibility,
};
use vessel_proto::{NodeId, WorkspaceId};

fn claim(layout: &mut Layout, key: &LayoutKey) -> PanelClaim {
    PanelClaim {
        workspace: key.clone(),
        tab: layout
            .open(
                Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )
            .unwrap(),
    }
}
fn key(node: NodeId) -> LayoutKey {
    LayoutKey::Workspace {
        node,
        workspace: WorkspaceId::random(),
    }
}

#[gpui::test]
fn hidden_and_moved_resources_keep_independent_actual_fields(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let residents = &mut shell.areas.session_residents;
        let mut layout = Layout::default();
        let node = NodeId::random();
        let key = key(node);
        let (a, b) = (SessionId::random(), SessionId::random());
        let (claim_a, claim_b) = (claim(&mut layout, &key), claim(&mut layout, &key));
        let mut build = |_: &SessionContext| SessionResources {
            session: SessionView::new(window, cx),
            conversation: ConversationArea::new(cx),
        };
        let token_a = residents
            .claim_session(
                SessionAddress { node, session: a },
                &claim_a,
                PaneId::MAIN,
                &mut build,
            )
            .unwrap();
        let token_b = residents
            .claim_session(
                SessionAddress { node, session: b },
                &claim_b,
                PaneId::MAIN,
                &mut build,
            )
            .unwrap();
        residents
            .present(token_a, &claim_a, PaneId::MAIN, Visibility::Visible)
            .unwrap();
        residents.focus(token_a, &claim_a).unwrap();
        let context_a = residents.context(token_a, &claim_a).unwrap();
        assert_eq!(residents.focused(), Some(context_a.clone()));
        let resources = residents.resources_mut(&context_a).unwrap();
        resources.session.composer.set_text("unsent A", window, cx);
        resources.conversation.open("original", a);
        residents
            .present(token_a, &claim_a, PaneId::MAIN, Visibility::RetainedHidden)
            .unwrap();
        residents
            .present(token_b, &claim_b, PaneId::MAIN, Visibility::Visible)
            .unwrap();
        residents.focus(token_b, &claim_b).unwrap();
        let context_b = residents.context(token_b, &claim_b).unwrap();
        residents
            .resources(&context_b)
            .unwrap()
            .session
            .composer
            .set_text("unsent B", window, cx);
        assert!(matches!(
            residents.resources(&context_a),
            Err(ResidentProblem::StaleEpoch)
        ));
        let side = layout
            .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
            .unwrap();
        residents
            .present(token_a, &claim_a, side, Visibility::Visible)
            .unwrap();
        residents.focus(token_a, &claim_a).unwrap();
        let moved = residents.context(token_a, &claim_a).unwrap();
        assert_eq!(moved.resident, token_a);
        assert_eq!(moved.pane, side);
        let resources = residents.resources(&moved).unwrap();
        assert_eq!(resources.session.composer.text(cx), "unsent A");
        assert_eq!(resources.conversation.shown.as_ref().unwrap().session, a);
        let b = residents.context(token_b, &claim_b).unwrap();
        assert_eq!(
            residents.resources(&b).unwrap().session.composer.text(cx),
            "unsent B"
        );
    });
}

#[gpui::test]
fn stale_and_duplicate_ingress_cannot_mutate_or_clear_a_successor(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let residents = &mut shell.areas.session_residents;
        let mut layout = Layout::default();
        let address = SessionAddress {
            node: NodeId::random(),
            session: SessionId::random(),
        };
        let claim = claim(&mut layout, &key(address.node));
        let token = residents
            .claim_session(address, &claim, PaneId::MAIN, |_| SessionResources {
                session: SessionView::new(window, cx),
                conversation: ConversationArea::new(cx),
            })
            .unwrap();
        let context = residents.context(token, &claim).unwrap();
        residents
            .resources(&context)
            .unwrap()
            .session
            .composer
            .set_text("protected input", window, cx);
        let first = residents.begin_follow(token, |_, _| {}).unwrap();
        let old_ticket = residents
            .admit_operation(first, OperationKind::Composer)
            .unwrap();
        let fresh = residents
            .begin_follow(token, |resources, retired| {
                assert_eq!(retired.binding(), first);
                assert_eq!(retired.operations().len(), 1);
                let operation = retired.operations().first().unwrap();
                assert_eq!(operation.kind(), OperationKind::Composer);
                assert_eq!(operation.ticket(), old_ticket);
                resources.session.resume_error = Some("uncertain".to_owned());
            })
            .unwrap();
        let resources = residents.resources(&context).unwrap();
        assert_eq!(resources.session.resume_error.as_deref(), Some("uncertain"));
        assert_eq!(resources.session.composer.text(cx), "protected input");
        let applied = Cell::new(false);
        assert!(residents.receive(first, |_| applied.set(true)).is_err());
        assert!(
            residents
                .settle(first, OperationKind::Composer, old_ticket, |_| applied
                    .set(true))
                .is_err()
        );
        assert!(!applied.get());
        // Fresh hidden stream events reach only their own resident.
        residents
            .receive(fresh, |resources| {
                resources.session.resume_error = Some("fresh".to_owned());
            })
            .unwrap();
        let ticket = residents
            .admit_operation(fresh, OperationKind::Timeline)
            .unwrap();
        residents
            .settle(fresh, OperationKind::Timeline, ticket, |_| {
                applied.set(true);
            })
            .unwrap();
        assert!(applied.get());
        applied.set(false);
        assert!(
            residents
                .settle(fresh, OperationKind::Timeline, ticket, |_| applied
                    .set(true))
                .is_err()
        );
        assert!(!applied.get());
        residents
            .retire_follow(fresh, |_, retired| {
                assert!(retired.operations().is_empty());
            })
            .unwrap();
        assert!(residents.receive(fresh, |_| applied.set(true)).is_err());
        assert!(matches!(
            residents.retire_clean(token),
            Err(ResidentProblem::Held)
        ));
        let resources = residents.resources(&context).unwrap();
        assert_eq!(resources.session.composer.text(cx), "protected input");
        resources.session.composer.clear(window, cx);
        for owner in ResourceOwner::ALL {
            residents
                .readiness(token, owner, OwnerReadiness::Ready)
                .unwrap();
        }
        residents
            .release_claim(token, &claim, Duration::ZERO)
            .unwrap();
        let resources = residents.retire_clean(token).unwrap();
        assert_eq!(resources.session.resume_error.as_deref(), Some("fresh"));
        assert!(residents.context(token, &claim).is_err());
        assert!(residents.receive(fresh, |_| applied.set(true)).is_err());
        assert!(!applied.get());
    });
}

#[gpui::test]
fn idempotence_and_capacity_refuse_before_allocating_actual_resources(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let residents = &mut shell.areas.session_residents;
        let mut layout = Layout::default();
        let node = NodeId::random();
        let key = key(node);
        let allocations = Cell::new(0_usize);
        let mut build = |_: &SessionContext| {
            allocations.set(allocations.get().saturating_add(1));
            SessionResources {
                session: SessionView::new(window, cx),
                conversation: ConversationArea::new(cx),
            }
        };
        let address = SessionAddress {
            node,
            session: SessionId::random(),
        };
        let first_claim = claim(&mut layout, &key);
        let token = residents
            .claim_session(address, &first_claim, PaneId::MAIN, &mut build)
            .unwrap();
        assert_eq!(
            residents
                .claim_session(address, &first_claim, PaneId::MAIN, &mut build)
                .unwrap(),
            token
        );
        assert_eq!(allocations.get(), 1);
        for _ in 1..vessel_app_view::residents::MAX_RESIDENT_SESSIONS {
            let _token = residents
                .claim_session(
                    SessionAddress {
                        node,
                        session: SessionId::random(),
                    },
                    &claim(&mut layout, &key),
                    PaneId::MAIN,
                    &mut build,
                )
                .unwrap();
        }
        let before = allocations.get();
        assert_eq!(
            residents.claim_session(
                SessionAddress {
                    node,
                    session: SessionId::random()
                },
                &claim(&mut layout, &key),
                PaneId::MAIN,
                &mut build
            ),
            Err(ResidentProblem::Capacity)
        );
        assert_eq!(allocations.get(), before);
    });
}

// A local legacy factory proves callback routing/field retention, not guarded
// transport, the concrete lifecycle owner or originating Window authority.
fn held_window_fixture(
    residents: &mut SessionResidents,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> (SessionContext, FollowBinding, StopWindowReceipt) {
    let address = SessionAddress {
        node: NodeId::random(),
        session: SessionId::random(),
    };
    let claim = claim(&mut Layout::default(), &key(address.node));
    let token = residents
        .claim_session(address, &claim, PaneId::MAIN, |_| SessionResources {
            session: SessionView::new(window, cx),
            conversation: ConversationArea::new(cx),
        })
        .unwrap();
    let context = residents.context(token, &claim).unwrap();
    residents
        .resources(&context)
        .unwrap()
        .session
        .composer
        .set_text("protected after ACK", window, cx);
    let binding = residents.begin_follow(token, |_, _| {}).unwrap();
    let ticket = residents
        .admission
        .admit_stop(binding, StopAction::Exit)
        .unwrap();
    let StopSettlement::WindowHeld(receipt) = residents
        .admission
        .settle_stop(binding, ticket, StopCompletion::Acknowledged)
        .unwrap()
    else {
        panic!("ACK must retain local Window work")
    };
    let _seal = residents.admission.end_follow(binding).unwrap();
    (context, binding, receipt)
}

#[gpui::test]
fn sealed_acknowledged_binding_notifies_full_retirement_even_with_no_wire_tickets(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let residents = &mut shell.areas.session_residents;
        let (context, binding, receipt) = held_window_fixture(residents, window, cx);
        let notified = Cell::new(false);
        residents
            .retire_follow(binding, |resources, notice| {
                assert_eq!(notice.binding(), binding);
                assert!(notice.operations().is_empty());
                assert_eq!(resources.session.composer.text(cx), "protected after ACK");
                notified.set(true);
            })
            .unwrap();
        assert!(notified.get());
        assert_eq!(
            residents.admission.validate_stop_window(binding, receipt),
            Err(ResidentProblem::StaleBinding)
        );
        assert!(
            residents
                .receive(binding, |_| panic!("retired ingress"))
                .is_err()
        );
        assert_eq!(
            residents
                .resources(&context)
                .unwrap()
                .session
                .composer
                .text(cx),
            "protected after ACK"
        );
    });
}
